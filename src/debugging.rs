use cosmic_text::LayoutRun;

#[allow(dead_code)]
pub fn debug_layout_run(run: &LayoutRun) -> String {
    format!(
        "layout run: line {}, top {}, height {}, of text: {}",
        run.line_i,
        run.line_top,
        run.line_height,
        text_preview(run.text)
    )
}

fn text_preview(text: &str) -> String {
    // Substring of 10 pixels
    let preview = text.chars().take(20).collect::<String>();

    format!("{}...", preview)
}
