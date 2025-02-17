use htmd::{Element, HtmlToMarkdown};
use markup5ever_rcdom::NodeData;
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
        content: Vec<Span>,
    },
    Paragraph {
        content: Vec<Span>,
    },
    List {
        items: Vec<ListItem>,
    },
    Image {
        alt_text: Option<String>,
        url: String,
    },
    BlockQuote {
        content: Vec<Block>,
    },
    ThematicBreak,
    CodeBlock {
        language: Option<String>,
        content: String,
    },
    Table {
        rows: Vec<TableRow>,
    },
}

#[derive(Debug, PartialEq)]
pub struct ListItem {
    pub marker: ListMarker,
    pub content: Vec<Block>,
}

#[derive(Debug, PartialEq)]
pub enum ListMarker {
    Bullet,
    Ordered { content: String },
}

#[derive(Debug, PartialEq)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

#[derive(Debug, PartialEq)]
pub struct TableCell {
    pub content: Vec<Span>,
}

#[derive(Debug, PartialEq, Clone)]
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

#[derive(Debug, PartialEq, Clone)]
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
        .add_handler(vec!["figcaption"], figcaption_handler)
        .add_handler(vec!["dt", "dd"], definition_list_handler)
        .add_handler(vec!["tbody"], table_body_handler)
        .add_handler(vec!["tr"], table_row_handler)
        .add_handler(vec!["td", "th"], table_cell_handler)
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

    let node_doc = tree.root_node();

    if node_doc.kind() != "document" {
        panic!("Expected root node to be a document");
    }

    let child_blocks = parse_child_blocks(&node_doc, source)?;

    let document = Document {
        blocks: child_blocks,
    };

    // info!("Parsed document: {:#?}", document);

    Ok(document)
}

fn figcaption_handler(element: Element) -> Option<String> {
    Some(format!("\n\n{}\n\n", element.content))
}

fn definition_list_handler(element: Element) -> Option<String> {
    Some(format!("{}\n\n", element.content))
}

fn table_body_handler(element: Element) -> Option<String> {
    let num_columns = count_table_columns(&element);

    if num_columns == 0 {
        Some(element.content.to_string())
    } else {
        let divider = "| - ".repeat(num_columns) + "|";

        Some(format!("{}\n{}", divider, element.content))
    }
}

fn count_table_columns(tbody_element: &Element) -> usize {
    let tbody_children = tbody_element.node.children.borrow();

    for child in tbody_children.iter() {
        if let NodeData::Element{ name: node_name, .. } = &child.data {
            if node_name.local.to_string() == "tr" {
                let first_row = child;
                let tr_children = first_row.children.borrow();

                let mut num_columns = 0;
                for tr_child in tr_children.iter() {
                    if let NodeData::Element{ name: node_name, .. } = &tr_child.data {
                        if node_name.local.to_string() == "td" {
                            num_columns += 1;
                        }
                    }
                }

                return num_columns;
            }
        }
    }

    0
}

fn table_row_handler(element: Element) -> Option<String> {
    Some(format!("{}|\n", element.content))
}

fn table_cell_handler(element: Element) -> Option<String> {
    Some(format!("| {} ", element.content))
}

fn parse_child_blocks(parent_block: &Node, source: &[u8]) -> Result<Vec<Block>, ParseError> {
    let mut cursor = parent_block.walk();
    let mut blocks = vec![];

    for node_block in parent_block.named_children(&mut cursor) {
        let block_result = parse_block(&node_block, source);
        if let Err(block_error) = block_result {
            return Err(block_error);
        }
        let block = block_result.unwrap();
        if let Some(block) = block {
            blocks.push(block);
        }
    }

    Ok(blocks)
}

fn parse_block(node_block: &Node, source: &[u8]) -> Result<Option<Block>, ParseError> {
    match node_block.kind() {
        "atx_heading" => parse_heading(node_block, source),
        "paragraph" => parse_paragraph(node_block, source),
        "tight_list" => parse_list(node_block, source),
        "loose_list" => parse_list(node_block, source),
        "block_quote" => parse_block_quote(node_block, source),
        "thematic_break" => Ok(Some(Block::ThematicBreak)),
        "fenced_code_block" => parse_code_block(node_block, source),
        "html_block" => Ok(Some(Block::Paragraph {
            content: vec![Span::Text {
                content: "(HTML block)".to_string(),
                style: SpanStyle::Normal,
            }],
        })),
        "list_marker" => Ok(None),
        "table" => parse_table(node_block, source),
        _ => Err(ParseError::UnexpectedNodeKind(
            node_block.kind().to_string(),
        )),
    }
}

fn parse_heading(node_heading: &Node, source: &[u8]) -> Result<Option<Block>, ParseError> {
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

    let moved = cursor.goto_next_sibling();
    if !moved {
        // No actual content in this heading, so skip it
        return Ok(None);
    }

    if cursor.node().kind() != "heading_content" {
        return Err(ParseError::UnexpectedNodeKind(
            cursor.node().kind().to_string(),
        ));
    }
    let node_heading_content = cursor.node();
    let mut spans = flatten_child_spans(&node_heading_content, &SpanStyle::Normal, source)?;

    // HACK: tree-sitter adds a leading space to the first span, so we trim it
    if let Some(first_span) = spans.first_mut() {
        if let Span::Text { content, .. } = first_span {
            *content = content.trim_start().to_string();
        }
    }

    Ok(Some(Block::Heading {
        level,
        content: spans,
    }))
}

fn parse_paragraph(node_paragraph: &Node, source: &[u8]) -> Result<Option<Block>, ParseError> {
    // If the paragraph contains an image and nothing else, create a Block::Image
    let mut cursor = node_paragraph.walk();
    let mut has_image = false;
    let mut has_text = false;
    for child in node_paragraph.named_children(&mut cursor) {
        match child.kind() {
            "image" => has_image = true,
            "text" => has_text = true,
            _ => {}
        }
    }

    if has_image && !has_text {
        return parse_image(node_paragraph, source);
    }

    let spans = flatten_child_spans(node_paragraph, &SpanStyle::Normal, source)?;

    Ok(Some(Block::Paragraph { content: spans }))
}

fn parse_image(node_paragraph: &Node, source: &[u8]) -> Result<Option<Block>, ParseError> {
    let mut cursor = node_paragraph.walk();
    let node_image = expect_node_kind(node_paragraph.named_child(0), "image")?;

    let node_link_destination = expect_node_kind(
        node_image
            .named_children(&mut cursor)
            .find(|child| child.kind() == "link_destination"),
        "link_destination",
    )?;
    let node_link_destination_inner_text =
        expect_node_kind(node_link_destination.named_child(0), "text")?;
    let url = node_link_destination_inner_text
        .utf8_text(source)?
        .to_string();

    let node_image_description = node_image
        .named_children(&mut cursor)
        .find(|child| child.kind() == "image_description");

    let alt_text: Option<String> = match node_image_description {
        Some(node_image_description) => {
            let node_image_description_inner_text =
                expect_node_kind(node_image_description.named_child(0), "text")?;

            Some(
                node_image_description_inner_text
                    .utf8_text(source)?
                    .to_string(),
            )
        }
        None => None,
    };

    Ok(Some(Block::Image { url, alt_text }))
}

fn parse_block_quote(node_block_quote: &Node, source: &[u8]) -> Result<Option<Block>, ParseError> {
    let child_blocks = parse_child_blocks(node_block_quote, source)?;

    Ok(Some(Block::BlockQuote {
        content: child_blocks,
    }))
}

fn parse_code_block(
    node_fenced_code_block: &Node,
    source: &[u8],
) -> Result<Option<Block>, ParseError> {
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

    let mut content: String = String::new();
    for child in node_code_fence_content.named_children(&mut cursor) {
        match child.kind() {
            "text" => {
                let node_text = expect_node_kind(Some(child), "text")?;
                content.push_str(node_text.utf8_text(source)?);
            }
            "line_break" => {
                content.push('\n');
            }
            _ => {
                return Err(ParseError::UnexpectedNodeKind(child.kind().to_string()));
            }
        }
    }

    Ok(Some(Block::CodeBlock { language, content }))
}

fn parse_list(node_list: &Node, source: &[u8]) -> Result<Option<Block>, ParseError> {
    let mut items: Vec<ListItem> = vec![];

    let mut cursor = node_list.walk();
    for node_list_item in node_list.named_children(&mut cursor) {
        let node_list_item = expect_node_kind(Some(node_list_item), "list_item")?;
        let item = parse_list_item(&node_list_item, source)?;
        items.push(item);
    }

    Ok(Some(Block::List { items }))
}

fn parse_list_item(node_list_item: &Node, source: &[u8]) -> Result<ListItem, ParseError> {
    let temp_marker = node_list_item.named_child(0).unwrap();
    let temp_marker_text = temp_marker.utf8_text(source)?.to_string();
    info!("List item marker: {}", temp_marker_text);

    let _node_list_marker = expect_node_kind(node_list_item.named_child(0), "list_marker")?;
    // TODO: pass the type of the list marker

    let child_blocks = parse_child_blocks(node_list_item, source)?;

    Ok(ListItem {
        marker: ListMarker::Bullet,
        content: child_blocks,
    })
}

fn parse_table(node_table: &Node, source: &[u8]) -> Result<Option<Block>, ParseError> {
    let mut rows: Vec<TableRow> = vec![];

    let mut cursor = node_table.walk();
    for node_row in node_table.named_children(&mut cursor) {
        let row = parse_table_row(&node_row, source)?;
        if let Some(row) = row {
            rows.push(row);
        }
    }

    Ok(Some(Block::Table { rows }))
}

fn parse_table_row(node_row: &Node, source: &[u8]) -> Result<Option<TableRow>, ParseError> {
    match node_row.kind() {
        "table_header_row" | "table_data_row" => (),
        _ => return Ok(None),
    }

    let mut cursor = node_row.walk();
    let mut cells: Vec<TableCell> = vec![];

    for node_cell in node_row.named_children(&mut cursor) {
        let cell = parse_table_cell(&node_cell, source)?;
        cells.push(cell);
    }

    Ok(Some(TableRow { cells }))
}

fn parse_table_cell(node_cell: &Node, source: &[u8]) -> Result<TableCell, ParseError> {
    if node_cell.kind() != "table_cell" {
        return Err(ParseError::WrongNodeKind(
            "table_cell".to_string(),
            node_cell.kind().to_string(),
        ));
    }

    let content = flatten_child_spans(node_cell, &SpanStyle::Normal, source)?;
    Ok(TableCell { content })
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
        "strong_emphasis" => flatten_child_spans(
            node_span,
            &merge_styles(parent_style, &SpanStyle::Bold),
            source,
        ),
        "emphasis" => flatten_child_spans(
            node_span,
            &merge_styles(parent_style, &SpanStyle::Italic),
            source,
        ),
        "code_span" => {
            let node_code_span_content = expect_node_kind(node_span.named_child(0), "text")?;
            let text = node_code_span_content.utf8_text(source)?.to_string();
            Ok(vec![Span::Text {
                content: text,
                style: SpanStyle::Code,
            }])
        }
        "backslash_escape" => {
            let text = node_span.utf8_text(source)?.to_string();
            let text = match text.as_str() {
                "\\[" => "[".to_string(),
                "\\]" => "]".to_string(),
                "\\_" => "_".to_string(),
                _ => format!("[TODO: handle backslash_escape content `{}`]", text),
            };
            info!("backslash escape {}", text);
            Ok(vec![Span::Text {
                content: text,
                style: parent_style.clone(),
            }])
        }
        "hard_line_break" => Ok(vec![Span::Text {
            content: "\n".to_string(),
            style: parent_style.clone(),
        }]),
        other_kind => {
            let text = format!("[TODO: parse node `{}`]", other_kind);
            Ok(vec![Span::Text {
                content: text,
                style: parent_style.clone(),
            }])
        }
    }
}

fn parse_link(node_link: &Node, source: &[u8]) -> Result<Link, ParseError> {
    let mut cursor = node_link.walk();

    let node_link_text = node_link
        .named_children(&mut cursor)
        .find(|child| child.kind() == "link_text");

    let text = match node_link_text {
        None => "(no link text)".to_string(),
        Some(node_link_text) => {
            if node_link_text.named_child_count() == 1
                && node_link_text.named_child(0).unwrap().kind() == "text"
            {
                let node_link_text_inner = expect_node_kind(node_link_text.named_child(0), "text")?;
                let text = node_link_text_inner.utf8_text(source)?.to_string();

                text
            } else {
                "(complex link contents)".to_string()
            }
        }
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

pub fn merge_styles(parent_style: &SpanStyle, new_style: &SpanStyle) -> SpanStyle {
    match (parent_style, new_style) {
        (&SpanStyle::Bold, SpanStyle::Italic) => SpanStyle::BoldItalic,
        (&SpanStyle::Italic, SpanStyle::Bold) => SpanStyle::BoldItalic,
        _ => new_style.clone(),
    }
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
    use pretty_assertions::assert_eq;

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
                        content: vec![Span::Text {
                            content: "My Document".to_string(),
                            style: SpanStyle::Normal,
                        }]
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
                        content: vec![Span::Text {
                            content: "My Document".to_string(),
                            style: SpanStyle::Normal,
                        }]
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

    #[test]
    fn test_simple_links() {
        let content = r#"
        <p>Here is a <a href="https://www.grovertoons.com/">link</a> and here is
        <a href="https://www.youtube.com/watch?v=dQw4w9WgXcQ">another one</a>.</p>
        "#;
        let input = create_html_document(content);
        let document = parse_webpage(&input).unwrap();

        assert_eq!(
            document,
            Document {
                blocks: vec![Block::Paragraph {
                    content: vec![
                        Span::Text {
                            content: "Here is a ".to_string(),
                            style: SpanStyle::Normal,
                        },
                        Span::Link(Link {
                            text: "link".to_string(),
                            destination: "https://www.grovertoons.com/".to_string(),
                        }),
                        Span::Text {
                            content: " and here is ".to_string(),
                            style: SpanStyle::Normal,
                        },
                        Span::Link(Link {
                            text: "another one".to_string(),
                            destination: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
                        }),
                        Span::Text {
                            content: ".".to_string(),
                            style: SpanStyle::Normal,
                        },
                    ],
                }]
            }
        );
    }

    #[test]
    fn test_inline_code() {
        let content = r#"
        <p>This paragraph contains some <code>inline_code()</code> so that's neat.</p>
        "#;
        let input = create_html_document(content);
        let document = parse_webpage(&input).unwrap();

        assert_eq!(
            document,
            Document {
                blocks: vec![Block::Paragraph {
                    content: vec![
                        Span::Text {
                            content: "This paragraph contains some ".to_string(),
                            style: SpanStyle::Normal,
                        },
                        Span::Text {
                            content: "inline_code()".to_string(),
                            style: SpanStyle::Code,
                        },
                        Span::Text {
                            content: " so that's neat.".to_string(),
                            style: SpanStyle::Normal,
                        },
                    ],
                }]
            }
        );
    }

    #[test]
    fn test_header_styles() {
        let content = r#"
        <h1>This header contains <em>styles</em> and <a href="https://example.com">a link</a>
        so that's neat.</h1>
        "#;
        let input = create_html_document(content);
        let document = parse_webpage(&input).unwrap();

        assert_eq!(
            document,
            Document {
                blocks: vec![Block::Heading {
                    level: 1,
                    content: vec![
                        Span::Text {
                            content: "This header contains ".to_string(),
                            style: SpanStyle::Normal,
                        },
                        Span::Text {
                            content: "styles".to_string(),
                            style: SpanStyle::Italic,
                        },
                        Span::Text {
                            content: " and ".to_string(),
                            style: SpanStyle::Normal,
                        },
                        Span::Link(Link {
                            text: "a link".to_string(),
                            destination: "https://example.com".to_string(),
                        }),
                        Span::Text {
                            content: " so that's neat.".to_string(),
                            style: SpanStyle::Normal,
                        },
                    ]
                }]
            }
        );
    }

    #[test]
    fn test_nested_styles() {
        let content = r#"
        <p>
            This is testing to make sure we can render nested styles, like
            some
            <em>italic text with <strong>bold</strong> nested inside</em>.
        </p>
        "#;
        let input = create_html_document(content);
        let document = parse_webpage(&input).unwrap();

        assert_eq!(
            document,
            Document {
                blocks: vec![Block::Paragraph {
                    content: vec![
                        Span::Text {
                            content: "This is testing to make sure we can render nested styles, like some ".to_string(),
                            style: SpanStyle::Normal,
                        },
                        Span::Text {
                            content: "italic text with ".to_string(),
                            style: SpanStyle::Italic,
                        },
                        Span::Text {
                            content: "bold".to_string(),
                            style: SpanStyle::BoldItalic,
                        },
                        Span::Text {
                            content: " nested inside".to_string(),
                            style: SpanStyle::Italic,
                        },
                        Span::Text {
                            content: ".".to_string(),
                            style: SpanStyle::Normal,
                        },
                    ]
                }]
            }
        )
    }

    #[test]
    fn test_code_block() {
        let content = r#"
    <p>Here is a code block:</p>
    <pre><code>fn main() {
    println!("Hello, world!");
}</code></pre>
        "#;
        let input = create_html_document(content);
        let document = parse_webpage(&input).unwrap();

        assert_eq!(
            document,
            Document {
                blocks: vec![
                    Block::Paragraph {
                        content: vec![Span::Text {
                            content: "Here is a code block:".to_string(),
                            style: SpanStyle::Normal,
                        }]
                    },
                    Block::CodeBlock {
                        language: None,
                        content: "fn main() {\n    println!(\"Hello, world!\");\n}".to_string(),
                    }
                ]
            }
        );
    }

    #[test]
    fn test_unordered_list() {
        let content = r#"
        <p>Here comes a list of animals:</p>
        <ul>
            <li>Cat</li>
            <li>Cat with some <em>style</em></li>
            <li>Dog
                <ul>
                    <li>Golden Retriever</li>
                    <li>Labrador</li>
                </ul>
            </li>
            <li>Crocodile</li>
        </ul>
        "#;
        let input = create_html_document(content);
        let document = parse_webpage(&input).unwrap();

        assert_eq!(
            document,
            Document {
                blocks: vec![
                    Block::Paragraph {
                        content: vec![Span::Text {
                            content: "Here comes a list of animals:".to_string(),
                            style: SpanStyle::Normal,
                        }]
                    },
                    Block::List {
                        items: vec![
                            ListItem {
                                marker: ListMarker::Bullet,
                                content: vec![Block::Paragraph {
                                    content: vec![Span::Text {
                                        content: "Cat".to_string(),
                                        style: SpanStyle::Normal,
                                    }]
                                }]
                            },
                            ListItem {
                                marker: ListMarker::Bullet,
                                content: vec![Block::Paragraph {
                                    content: vec![
                                        Span::Text {
                                            content: "Cat with some ".to_string(),
                                            style: SpanStyle::Normal,
                                        },
                                        Span::Text {
                                            content: "style".to_string(),
                                            style: SpanStyle::Italic,
                                        },
                                    ]
                                }]
                            },
                            ListItem {
                                marker: ListMarker::Bullet,
                                content: vec![
                                    Block::Paragraph {
                                        content: vec![Span::Text {
                                            content: "Dog".to_string(),
                                            style: SpanStyle::Normal,
                                        }]
                                    },
                                    Block::List {
                                        items: vec![
                                            ListItem {
                                                marker: ListMarker::Bullet,
                                                content: vec![Block::Paragraph {
                                                    content: vec![Span::Text {
                                                        content: "Golden Retriever".to_string(),
                                                        style: SpanStyle::Normal,
                                                    }]
                                                }]
                                            },
                                            ListItem {
                                                marker: ListMarker::Bullet,
                                                content: vec![Block::Paragraph {
                                                    content: vec![Span::Text {
                                                        content: "Labrador".to_string(),
                                                        style: SpanStyle::Normal,
                                                    }]
                                                }]
                                            },
                                        ]
                                    },
                                ]
                            },
                            ListItem {
                                marker: ListMarker::Bullet,
                                content: vec![Block::Paragraph {
                                    content: vec![Span::Text {
                                        content: "Crocodile".to_string(),
                                        style: SpanStyle::Normal,
                                    }]
                                }]
                            },
                        ],
                    }
                ]
            }
        );
    }

    #[test]
    fn test_block_image() {
        let content = r#"
        <p>Here is an image of a cat:</p>
        <img src="https://www.example.com/cat.jpg" alt="A cat" />
        <p>This image has no alt text:</p>
        <img src="https://www.example.com/cat.jpg" />
        "#;
        let input = create_html_document(content);
        let document = parse_webpage(&input).unwrap();

        assert_eq!(
            document,
            Document {
                blocks: vec![
                    Block::Paragraph {
                        content: vec![Span::Text {
                            content: "Here is an image of a cat:".to_string(),
                            style: SpanStyle::Normal,
                        }]
                    },
                    Block::Image {
                        url: "https://www.example.com/cat.jpg".to_string(),
                        alt_text: Some("A cat".to_string()),
                    },
                    Block::Paragraph {
                        content: vec![Span::Text {
                            content: "This image has no alt text:".to_string(),
                            style: SpanStyle::Normal,
                        }]
                    },
                    Block::Image {
                        url: "https://www.example.com/cat.jpg".to_string(),
                        alt_text: None,
                    }
                ]
            }
        );
    }

    #[test]
    fn test_image_link() {
        let content = r#"
        <p>Click on this image:</p>

        <p><a href="https://example.com"><img src="https://www.example.com/cat.jpg" alt="A cat"></a></p>
        "#;
        let input = create_html_document(content);
        let document = parse_webpage(&input).unwrap();

        assert_eq!(
            document,
            Document {
                blocks: vec![
                    Block::Paragraph {
                        content: vec![Span::Text {
                            content: "Click on this image:".to_string(),
                            style: SpanStyle::Normal,
                        }]
                    },
                    Block::Paragraph {
                        content: vec![Span::Link(Link {
                            text: "(complex link contents)".to_string(),
                            destination: "https://example.com".to_string(),
                        })]
                    }
                ]
            }
        );
    }

    #[test]
    fn test_definition_list() {
        let content = r#"
        <dl>
            <dt>Term 1</dt>
            <dd>Definition 1</dd>
            <dt>Term 2 with <em>styles</em> inside</dt>
            <dd>Definition 2 with <em>styles</em> inside</dd>
        </dl>
        "#;
        let input = create_html_document(content);
        let document = parse_webpage(&input).unwrap();

        assert_eq!(
            document,
            Document {
                blocks: vec![
                    Block::Paragraph {
                        content: vec![Span::Text {
                            content: "Term 1".to_string(),
                            style: SpanStyle::Normal,
                        },]
                    },
                    Block::Paragraph {
                        content: vec![Span::Text {
                            content: "Definition 1".to_string(),
                            style: SpanStyle::Normal,
                        },]
                    },
                    Block::Paragraph {
                        content: vec![
                            Span::Text {
                                content: "Term 2 with ".to_string(),
                                style: SpanStyle::Normal,
                            },
                            Span::Text {
                                content: "styles".to_string(),
                                style: SpanStyle::Italic,
                            },
                            Span::Text {
                                content: " inside".to_string(),
                                style: SpanStyle::Normal,
                            },
                        ]
                    },
                    Block::Paragraph {
                        content: vec![
                            Span::Text {
                                content: "Definition 2 with ".to_string(),
                                style: SpanStyle::Normal,
                            },
                            Span::Text {
                                content: "styles".to_string(),
                                style: SpanStyle::Italic,
                            },
                            Span::Text {
                                content: " inside".to_string(),
                                style: SpanStyle::Normal,
                            },
                        ]
                    },
                ]
            }
        );
    }

    #[test]
    fn test_figure() {
        let content = r#"
        <p>Consider the following:</p>
        <figure>
            <img src="https://www.example.com/cat.jpg" alt="A cat" />
            <figcaption>An image of a cat</figcaption>
        </figure>
        <p>As you can see, that was a cat.</p>
        "#;
        let input = create_html_document(content);
        let document = parse_webpage(&input).unwrap();

        assert_eq!(
            document,
            Document {
                blocks: vec![
                    Block::Paragraph {
                        content: vec![Span::Text {
                            content: "Consider the following:".to_string(),
                            style: SpanStyle::Normal
                        }]
                    },
                    Block::Image {
                        url: "https://www.example.com/cat.jpg".to_string(),
                        alt_text: Some("A cat".to_string())
                    },
                    Block::Paragraph {
                        content: vec![Span::Text {
                            content: "An image of a cat".to_string(),
                            style: SpanStyle::Normal
                        }]
                    },
                    Block::Paragraph {
                        content: vec![Span::Text {
                            content: "As you can see, that was a cat.".to_string(),
                            style: SpanStyle::Normal
                        }]
                    },
                ]
            }
        );
    }
}
