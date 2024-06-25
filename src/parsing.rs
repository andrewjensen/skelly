use thiserror::Error;
use tree_sitter::{Node, Parser};

#[derive(Debug)]
pub struct Document {
    pub blocks: Vec<Block>,
}

#[derive(Debug)]
pub enum Block {
    Heading { level: u8, content: String },
    Paragraph { content: String },
    List,
}

#[derive(Debug)]
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
}

pub fn parse_webpage(page_html: &str) -> Result<Document, ParseError> {
    // HACK: we're parsing from HTML to markdown, then parsing that markdown
    // We should eventually consolidate and just work with a single intermediate representation
    let page_markdown = htmd::convert(page_html);
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
    let content = temp_squash_block_text(&node_heading_content, source);

    Ok(Block::Heading { level, content })
}

fn parse_paragraph(node_paragraph: &Node, source: &[u8]) -> Result<Block, ParseError> {
    Ok(Block::Paragraph {
        content: temp_squash_block_text(node_paragraph, source),
    })
}

fn temp_squash_block_text(node_parent: &Node, source: &[u8]) -> String {
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
                content.push_str(node_child.utf8_text(source).unwrap());
            }
            "link" => {
                let link = parse_link(&node_child, source);
                content.push_str(&format!("[{}]({})", link.text, link.destination));
            }
            "strong_emphasis" => {
                let node_emphasized_text = node_child.named_child(0).expect("Expected text node");
                let emphasized_text = node_emphasized_text.utf8_text(source).unwrap();
                content.push_str(&format!("**{}**", emphasized_text));
            }
            _ => {
                content.push_str(&format!("[TODO: handle node `{}`]", node_child.kind()));
            }
        }
    }

    content.trim().to_string()
}

fn parse_link(node_link: &Node, source: &[u8]) -> Link {
    let node_link_text = node_link.named_child(0).expect("Expected link_text node");
    let node_link_text_inner = node_link_text
        .named_child(0)
        .expect("Expected text node inside of link_text");
    let text = node_link_text_inner.utf8_text(source).unwrap().to_string();

    let node_link_destination = node_link
        .named_child(1)
        .expect("Expected link_destination node");
    let node_link_destination_inner = node_link_destination
        .named_child(0)
        .expect("Expected text node inside of link_destination");
    let destination = node_link_destination_inner
        .utf8_text(source)
        .unwrap()
        .to_string();

    Link { destination, text }
}
