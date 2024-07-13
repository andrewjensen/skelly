use htmd::{Element, HtmlToMarkdown};
use log::info;
use thiserror::Error;
use tree_sitter::{Node, Parser};

#[derive(Debug, PartialEq)]
pub struct Document {
    pub blocks: Vec<Block>,
}

#[derive(Debug, PartialEq)]
pub enum Block {
    Heading {
        level: u8,
        content: String,
    },
    Paragraph {
        content: Vec<Span>,
    },
    List,
    BlockQuote {
        content: String,
    },
    ThematicBreak,
    CodeBlock {
        language: Option<String>,
        content: String,
    },
}

#[derive(Debug, PartialEq)]
pub enum Span {
    Text { content: String, style: SpanStyle },
    Link(Link),
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpanStyle {
    Normal,
    Bold,
    Italic,
    BoldItalic,
    Code,
}

#[derive(Debug, PartialEq)]
pub struct Link {
    pub destination: String,
    pub text: String,
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("HTMD error")]
    HtmdError,

    #[error("TreeSitter error")]
    TreeSitterError,

    #[error("Encountered unexpected node kind: {0}")]
    UnexpectedNodeKind(String),

    #[error("Wrong node kind: Expected {0} but received {1}")]
    WrongNodeKind(String, String),

    #[error("Missing expected node kind: {0}")]
    MissingExpectedNodeKind(String),

    #[error("Failed to get node text: {0}")]
    FailedToGetNodeText(#[from] std::str::Utf8Error),
}

pub fn parse_webpage(page_html: &str) -> Result<Document, ParseError> {
    // HACK: we're parsing from HTML to markdown, then parsing that markdown
    // We should eventually consolidate and just work with a single intermediate representation

    let converter = HtmlToMarkdown::builder()
        .add_handler(vec!["script", "style", "title"], |_: Element| None)
        .build();
    let page_markdown = converter.convert(page_html);
    if page_markdown.is_err() {
        return Err(ParseError::HtmdError);
    }
    let page_markdown = page_markdown.unwrap();
    let source = page_markdown.as_bytes();

    // info!("Page content as markdown: {}", page_markdown);

    let markdown_language = tree_sitter_markdown::language();

    let mut parser = Parser::new();
    parser
        .set_language(markdown_language)
        .expect("Error loading Markdown grammar");

    let tree_sitter_parse_result = parser.parse(source, None);
    if tree_sitter_parse_result.is_none() {
        return Err(ParseError::TreeSitterError);
    }
    let tree = tree_sitter_parse_result.unwrap();

    // info!("Tree: {}", tree.root_node().to_sexp());

    let mut blocks = vec![];

    let node_doc = tree.root_node();

    if node_doc.kind() != "document" {
        panic!("Expected root node to be a document");
    }

    let mut cursor = node_doc.walk();
    for node_block in node_doc.named_children(&mut cursor) {
        let block_result = parse_block(&node_block, source);
        if let Err(block_error) = block_result {
            return Err(block_error);
        }
        let block = block_result.unwrap();
        blocks.push(block);
    }

    Ok(Document { blocks })
}

fn parse_block(node_block: &Node, source: &[u8]) -> Result<Block, ParseError> {
    match node_block.kind() {
        "atx_heading" => parse_heading(node_block, source),
        "paragraph" => parse_paragraph(node_block, source),
        "tight_list" => Ok(Block::List),
        "loose_list" => Ok(Block::List),
        "block_quote" => parse_block_quote(node_block, source),
        "thematic_break" => Ok(Block::ThematicBreak),
        "fenced_code_block" => parse_code_block(node_block, source),
        _ => Err(ParseError::UnexpectedNodeKind(
            node_block.kind().to_string(),
        )),
    }
}

fn parse_heading(node_heading: &Node, source: &[u8]) -> Result<Block, ParseError> {
    let mut cursor = node_heading.walk();

    cursor.goto_first_child();

    let heading_level_marker = cursor.node().kind();
    let level = match heading_level_marker {
        "atx_h1_marker" => Some(1),
        "atx_h2_marker" => Some(2),
        "atx_h3_marker" => Some(3),
        "atx_h4_marker" => Some(4),
        "atx_h5_marker" => Some(5),
        "atx_h6_marker" => Some(6),
        _ => None,
    };
    if level.is_none() {
        return Err(ParseError::UnexpectedNodeKind(
            heading_level_marker.to_string(),
        ));
    }
    let level = level.unwrap();

    cursor.goto_next_sibling();
    if cursor.node().kind() != "heading_content" {
        return Err(ParseError::UnexpectedNodeKind(
            cursor.node().kind().to_string(),
        ));
    }
    let node_heading_content = cursor.node();
    let content = temp_squash_block_text(&node_heading_content, source)?;

    Ok(Block::Heading { level, content })
}

fn parse_paragraph(node_paragraph: &Node, source: &[u8]) -> Result<Block, ParseError> {
    let spans = flatten_child_spans(node_paragraph, &SpanStyle::Normal, source)?;

    Ok(Block::Paragraph { content: spans })
}

fn parse_block_quote(node_block_quote: &Node, source: &[u8]) -> Result<Block, ParseError> {
    let content = temp_squash_block_text(node_block_quote, source)?;
    Ok(Block::BlockQuote { content })
}

fn parse_code_block(node_fenced_code_block: &Node, source: &[u8]) -> Result<Block, ParseError> {
    let mut cursor = node_fenced_code_block.walk();

    let node_info_string = node_fenced_code_block
        .named_children(&mut cursor)
        .find(|child| child.kind() == "info_string");
    let language = match node_info_string {
        Some(node_info_string) => {
            let node_text = expect_node_kind(node_info_string.named_child(0), "text")?;
            let language = node_text.utf8_text(source)?.to_string();

            Some(language)
        }
        None => None,
    };

    let node_code_fence_content = expect_node_kind(
        node_fenced_code_block
            .named_children(&mut cursor)
            .find(|child| child.kind() == "code_fence_content"),
        "code_fence_content",
    )?;
    let node_content_inner_text = expect_node_kind(node_code_fence_content.named_child(0), "text")?;
    let content = node_content_inner_text.utf8_text(source)?.to_string();

    Ok(Block::CodeBlock { language, content })
}

fn flatten_child_spans(
    node_parent: &Node,
    parent_style: &SpanStyle,
    source: &[u8],
) -> Result<Vec<Span>, ParseError> {
    let mut overall_spans = vec![];

    let mut cursor = node_parent.walk();
    for node_child in node_parent.named_children(&mut cursor) {
        let spans = parse_span(&node_child, parent_style, source)?;
        for span in spans {
            overall_spans.push(span);
        }
    }

    Ok(overall_spans)
}

fn parse_span(
    node_span: &Node,
    parent_style: &SpanStyle,
    source: &[u8],
) -> Result<Vec<Span>, ParseError> {
    match node_span.kind() {
        "text" => {
            let text = node_span.utf8_text(source)?.to_string();
            Ok(vec![Span::Text {
                content: text,
                style: parent_style.clone(),
            }])
        }
        "link" => {
            let link = parse_link(node_span, source)?;
            Ok(vec![Span::Link(link)])
        }
        "strong_emphasis" => flatten_child_spans(node_span, &SpanStyle::Bold, source),
        "emphasis" => flatten_child_spans(node_span, &SpanStyle::Italic, source),
        other_kind => {
            let text = format!("[TODO: parse node `{}`]", other_kind);
            Ok(vec![Span::Text {
                content: text,
                style: parent_style.clone(),
            }])
        }
    }
}

fn temp_squash_block_text(node_parent: &Node, source: &[u8]) -> Result<String, ParseError> {
    let mut content = String::new();

    let mut first = true;
    let mut cursor = node_parent.walk();
    for node_child in node_parent.named_children(&mut cursor) {
        if !first {
            content.push_str(" ");
        }
        first = false;

        match node_child.kind() {
            "text" => {
                let node_text = node_child.utf8_text(source)?;
                content.push_str(node_text);
            }
            "link" => {
                let link = parse_link(&node_child, source)?;
                content.push_str(&format!("[{}]({})", link.text, link.destination));
            }
            "strong_emphasis" => {
                let node_emphasized_text = node_child.named_child(0).expect("Expected text node");
                let emphasized_text = node_emphasized_text.utf8_text(source)?;
                content.push_str(&format!("**{}**", emphasized_text));
            }
            _ => {
                content.push_str(&format!("[TODO: handle node `{}`]", node_child.kind()));
            }
        }
    }

    Ok(content.trim().to_string())
}

fn parse_link(node_link: &Node, source: &[u8]) -> Result<Link, ParseError> {
    let mut cursor = node_link.walk();

    let node_link_text = node_link
        .named_children(&mut cursor)
        .find(|child| child.kind() == "link_text");

    let text: String = match node_link_text {
        None => "(No link text)".to_string(),
        Some(node_link_text) => match node_link_text.kind() {
            "text" => node_link_text.utf8_text(source)?.to_string(),
            _ => "(Complex link text)".to_string(),
        },
    };

    let node_link_destination = expect_node_kind(
        node_link
            .named_children(&mut cursor)
            .find(|child| child.kind() == "link_destination"),
        "link_destination",
    )?;

    let node_link_destination_inner =
        expect_node_kind(node_link_destination.named_child(0), "text")?;
    let destination = node_link_destination_inner.utf8_text(source)?.to_string();

    Ok(Link { destination, text })
}

fn expect_node_kind<'s, 'n>(
    node: Option<Node<'n>>,
    expected_kind: &str,
) -> Result<Node<'n>, ParseError> {
    match node {
        None => Err(ParseError::MissingExpectedNodeKind(
            expected_kind.to_string(),
        )),
        Some(node) => {
            if node.kind() != expected_kind {
                Err(ParseError::WrongNodeKind(
                    expected_kind.to_string(),
                    node.kind().to_string(),
                ))
            } else {
                Ok(node)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn create_html_document(inner_content: &str) -> String {
        format!("<!doctype html><html><head><title>Document</title></head><body><article>{}</article></body></html>", inner_content)
    }

    #[test]
    fn test_parse_simple() {
        let content = r#"
        <h1>My Document</h1>
        <p>This is a paragraph.</p>
        <p>This is another paragraph.</p>
        "#;
        let input = create_html_document(content);
        let document = parse_webpage(&input).unwrap();

        assert_eq!(
            document,
            Document {
                blocks: vec![
                    Block::Heading {
                        level: 1,
                        content: "My Document".to_string()
                    },
                    Block::Paragraph {
                        content: vec![Span::Text {
                            content: "This is a paragraph.".to_string(),
                            style: SpanStyle::Normal,
                        }],
                    },
                    Block::Paragraph {
                        content: vec![Span::Text {
                            content: "This is another paragraph.".to_string(),
                            style: SpanStyle::Normal,
                        }],
                    }
                ]
            }
        );
    }

    #[test]
    fn test_parse_inline_styles() {
        let content = r#"
        <h1>My Document</h1>
        <p>This is a paragraph containing <strong>lots</strong> of exciting <em>styles</em>.</p>
        "#;
        let input = create_html_document(content);
        let document = parse_webpage(&input).unwrap();

        assert_eq!(
            document,
            Document {
                blocks: vec![
                    Block::Heading {
                        level: 1,
                        content: "My Document".to_string()
                    },
                    Block::Paragraph {
                        content: vec![
                            Span::Text {
                                content: "This is a paragraph containing ".to_string(),
                                style: SpanStyle::Normal,
                            },
                            Span::Text {
                                content: "lots".to_string(),
                                style: SpanStyle::Bold,
                            },
                            Span::Text {
                                content: " of exciting ".to_string(),
                                style: SpanStyle::Normal,
                            },
                            Span::Text {
                                content: "styles".to_string(),
                                style: SpanStyle::Italic,
                            },
                            Span::Text {
                                content: ".".to_string(),
                                style: SpanStyle::Normal,
                            },
                        ],
                    }
                ]
            }
        );
    }
}
