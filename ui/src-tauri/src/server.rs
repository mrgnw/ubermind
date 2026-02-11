use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Json;
use axum::Router;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::services;
use crate::tmux;

pub const DEFAULT_PORT: u16 = 13369;

pub async fn start(port: u16, static_dir: Option<PathBuf>) {
	let api = Router::new()
		.route("/api/services", get(api_services))
		.route("/api/services/{name}", get(api_service_detail))
		.route("/api/services/{name}/start", post(api_start))
		.route("/api/services/{name}/stop", post(api_stop))
		.route("/api/services/{name}/reload", post(api_reload))
		.route("/api/services/{name}/echo", get(api_echo))
		.route("/api/services/{name}/panes", get(api_panes))
		.route(
			"/api/services/{name}/panes/{window}/{pane}",
			get(api_capture_pane),
		)
		.route("/ws/echo/{name}", get(ws_echo))
		.layer(CorsLayer::permissive());

	let app = if let Some(dir) = static_dir {
		api.fallback_service(ServeDir::new(dir).append_index_html_on_directories(true))
	} else {
		api
	};

	let addr = SocketAddr::from(([0, 0, 0, 0], port));
	log::info!("HTTP server listening on http://{addr}");

	let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
	axum::serve(listener, app).await.unwrap();
}

async fn api_services() -> Json<Vec<services::ServiceInfo>> {
	Json(services::list_services())
}

async fn api_service_detail(Path(name): Path<String>) -> impl IntoResponse {
	match services::get_service_detail(&name) {
		Ok(detail) => (StatusCode::OK, Json(serde_json::to_value(detail).unwrap())),
		Err(e) => (
			StatusCode::NOT_FOUND,
			Json(serde_json::json!({ "error": e })),
		),
	}
}

async fn api_start(Path(name): Path<String>) -> impl IntoResponse {
	match services::start_service(&name) {
		Ok(msg) => (StatusCode::OK, Json(serde_json::json!({ "message": msg }))),
		Err(e) => (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(serde_json::json!({ "error": e })),
		),
	}
}

async fn api_stop(Path(name): Path<String>) -> impl IntoResponse {
	match services::stop_service(&name) {
		Ok(msg) => (StatusCode::OK, Json(serde_json::json!({ "message": msg }))),
		Err(e) => (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(serde_json::json!({ "error": e })),
		),
	}
}

async fn api_reload(Path(name): Path<String>) -> impl IntoResponse {
	match services::reload_service(&name) {
		Ok(msg) => (StatusCode::OK, Json(serde_json::json!({ "message": msg }))),
		Err(e) => (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(serde_json::json!({ "error": e })),
		),
	}
}

async fn api_echo(Path(name): Path<String>) -> impl IntoResponse {
	match tmux::capture_all_panes(&name) {
		Ok(content) => (StatusCode::OK, content),
		Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
	}
}

async fn api_panes(Path(name): Path<String>) -> Json<Vec<tmux::TmuxPane>> {
	Json(tmux::list_panes(&name))
}

async fn api_capture_pane(Path((name, window, pane)): Path<(String, u32, u32)>) -> impl IntoResponse {
	match tmux::capture_pane(&name, window, pane) {
		Ok(content) => (StatusCode::OK, content),
		Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
	}
}

async fn ws_echo(
	Path(name): Path<String>,
	ws: WebSocketUpgrade,
) -> impl IntoResponse {
	ws.on_upgrade(move |socket| handle_echo_ws(socket, name))
}

async fn handle_echo_ws(mut socket: WebSocket, name: String) {
	let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(200));
	let mut last_content = String::new();

	loop {
		tokio::select! {
			_ = interval.tick() => {
				match tmux::capture_all_panes(&name) {
					Ok(content) => {
						if content != last_content {
							last_content = content.clone();
							if socket.send(Message::Text(content.into())).await.is_err() {
								break;
							}
						}
					}
					Err(_) => {}
				}
			}
			msg = socket.recv() => {
				match msg {
					Some(Ok(Message::Close(_))) | None => break,
					_ => {}
				}
			}
		}
	}
}
