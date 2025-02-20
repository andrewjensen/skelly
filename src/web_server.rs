#![allow(dead_code)]

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use http_body_util::BodyExt;
use log::info;

use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tokio::runtime::Builder;
use tokio::sync::mpsc::Sender as TokioSender;
use tokio::sync::mpsc::channel as tokio_channel;
use std::sync::mpsc::Sender as StdSender;

use crate::application::{UserInputEvent, NavigateCommand, RenderCommand};

struct ServerState {
    input_internal_tx: TokioSender<UserInputEvent>,
}

pub fn run_web_server(user_input_tx: StdSender<UserInputEvent>) {
    // TRICKY: The rest of the app is sync, but we need async for the web server.
    // We use an internal _tokio_ channel, then relay it to the main thread via a _std_ channel.

    let (input_internal_tx, mut input_internal_rx) = tokio_channel::<UserInputEvent>(32);

    // let input_tx_shared = input_internal_tx.clone();
    let shared_server_state = Arc::new(ServerState {
        input_internal_tx,
    });

    // Start a tokio runtime just for the web server
    let tokio_runtime = Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    let web_server_handle = tokio_runtime.spawn(async move {
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

    info!("Web server started");

    std::thread::spawn(move || {
        while let Some(input_event) = input_internal_rx.blocking_recv() {
            user_input_tx.send(input_event).unwrap();
        }
    });

    tokio_runtime.block_on(web_server_handle).unwrap();
}

async fn serve_web_ui() -> Html<String> {
    let html = include_bytes!("../assets/web_ui.html");
    let html_string: String = String::from_utf8_lossy(html).to_string();

    Html(html_string)
}

async fn handle_navigate_command(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<NavigateCommand>,
) -> Response {
    state
        .input_internal_tx
        .send(UserInputEvent::Navigate(payload.clone()))
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
        .input_internal_tx
        .send(UserInputEvent::Render(render_command))
        .await
        .unwrap();

    return Response::builder()
        .status(StatusCode::OK)
        .body(Body::empty())
        .unwrap();
}
