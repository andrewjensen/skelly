use tower_http::cors::CorsLayer;

struct ServerState {
    app_tx: Sender<AppEvent>,
}

#[derive(Clone, Debug, Deserialize)]
struct NavigateCommand {
    pub url: String,
}

#[derive(Debug)]
struct RenderCommand {
    pub html: String,
    // Needed for resolving relative image URLs
    pub page_url: String,
}

pub async fn run_web_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    let shared_server_state = Arc::new(ServerState {
        app_tx: app_tx_shared,
    });
    let web_server_join = tokio::spawn(async move {
        info!("Starting web server...");
        let web_server = Router::new()
            .route("/", get(serve_web_ui))
            .route("/navigate", post(handle_navigate_command))
            .route("/render", post(handle_render_command))
            .layer(CorsLayer::permissive())
            .with_state(shared_server_state);
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        axum::serve(listener, web_server).await.unwrap();
    });

}

async fn serve_web_ui() -> Html<String> {
    let html = include_bytes!("../../assets/web_ui.html");
    let html_string: String = String::from_utf8_lossy(html).to_string();

    Html(html_string)
}

async fn handle_navigate_command(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<NavigateCommand>,
) -> Response {
    state
        .app_tx
        .send(AppEvent::Navigate(payload.clone()))
        .await
        .unwrap();

    Response::builder()
        .status(StatusCode::OK)
        .body(Body::empty())
        .unwrap()
}

async fn handle_render_command(State(state): State<Arc<ServerState>>, req: Request) -> Response {
    let (parts, body) = req.into_parts();

    let headers = parts.headers;

    let page_url = match headers.get("x-skelly-page-url") {
        Some(page_url) => page_url.to_str().unwrap().to_string(),
        None => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::empty())
                .unwrap();
        }
    };

    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::empty())
                .unwrap();
        }
    };
    let body_str: String = match std::str::from_utf8(&bytes) {
        Ok(body) => body.to_string(),
        Err(_err) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::empty())
                .unwrap();
        }
    };

    let render_command = RenderCommand {
        html: body_str,
        page_url,
    };
    state
        .app_tx
        .send(AppEvent::Render(render_command))
        .await
        .unwrap();

    return Response::builder()
        .status(StatusCode::OK)
        .body(Body::empty())
        .unwrap();
}
