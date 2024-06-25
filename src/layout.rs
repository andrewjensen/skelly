use cosmic_text::LayoutRun;

pub struct LayoutPage<'a> {
    pub runs: Vec<&'a LayoutRun<'a>>,
    pub offset: f32,
}

pub fn split_runs_into_pages<'a>(
    all_runs: Vec<&'a LayoutRun<'a>>,
    content_height: u32,
) -> Vec<LayoutPage<'a>> {
    let mut pages: Vec<LayoutPage> = vec![];
    let mut current_page = LayoutPage {
        runs: vec![],
        offset: 0.0,
    };

    for run in all_runs.into_iter() {
        let run_bottom = run.line_top + run.line_height - current_page.offset;
        if run_bottom > content_height as f32 {
            // Finish up this page and start a new one

            current_page.offset = current_page.runs.first().unwrap().line_top;
            pages.push(current_page);

            current_page = LayoutPage {
                runs: vec![],
                offset: run.line_top,
            };
        }

        current_page.runs.push(run);
    }

    pages.push(current_page);

    pages
}
