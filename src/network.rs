use log::error;

pub fn fetch_webpage(url: &str) -> String {
    let client = reqwest::blocking::Client::new();
    let request = client.get(url).header("Accept", "text/html");
    let response = request.send().unwrap();

    if response.status().is_success() {
        let body = response.text().unwrap();

        body.to_string()
    } else {
        error!("Failed to fetch webpage: {}", response.status().as_u16());

        panic!("Failed to fetch webpage");
    }
}
