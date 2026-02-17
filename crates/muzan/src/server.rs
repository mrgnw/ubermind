use std::future::Future;
use std::sync::Arc;
use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

use crate::paths::DaemonPaths;

/// Run a Unix socket server that deserializes requests, passes them to `handler`,
/// and writes back serialized responses. Newline-delimited JSON protocol.
///
/// Parse errors are silently dropped. Use [`run_socket_server_with_error`] to send
/// error responses for malformed requests.
pub async fn run_socket_server<Req, Resp, F, Fut>(
	paths: &DaemonPaths,
	handler: F,
) where
	Req: DeserializeOwned + Send + 'static,
	Resp: Serialize + Send + 'static,
	F: Fn(Req) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Resp> + Send,
{
	run_socket_server_with_error(paths, handler, None::<fn(String) -> Resp>).await;
}

/// Like [`run_socket_server`], but with an optional callback for parse errors.
///
/// When a request can't be deserialized, `on_parse_error` is called with the error
/// message and its return value is sent back to the client.
pub async fn run_socket_server_with_error<Req, Resp, F, Fut, E>(
	paths: &DaemonPaths,
	handler: F,
	on_parse_error: Option<E>,
) where
	Req: DeserializeOwned + Send + 'static,
	Resp: Serialize + Send + 'static,
	F: Fn(Req) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Resp> + Send,
	E: Fn(String) -> Resp + Send + Sync + 'static,
{
	let socket_path = paths.socket_path();

	let listener = match UnixListener::bind(&socket_path) {
		Ok(l) => l,
		Err(e) => {
			tracing::error!("failed to bind socket {}: {}", socket_path.display(), e);
			return;
		}
	};

	tracing::info!("listening on {}", socket_path.display());

	let handler = Arc::new(handler);
	let on_parse_error = on_parse_error.map(Arc::new);

	loop {
		let (stream, _) = match listener.accept().await {
			Ok(s) => s,
			Err(e) => {
				tracing::error!("accept error: {}", e);
				continue;
			}
		};

		let handler = Arc::clone(&handler);
		let on_parse_error = on_parse_error.clone();
		tokio::spawn(async move {
			handle_connection(stream, handler, on_parse_error).await;
		});
	}
}

async fn handle_connection<Req, Resp, F, Fut, E>(
	stream: tokio::net::UnixStream,
	handler: Arc<F>,
	on_parse_error: Option<Arc<E>>,
) where
	Req: DeserializeOwned + Send + 'static,
	Resp: Serialize + Send + 'static,
	F: Fn(Req) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Resp> + Send,
	E: Fn(String) -> Resp + Send + Sync + 'static,
{
	let (reader, mut writer) = stream.into_split();
	let mut lines = BufReader::new(reader).lines();

	while let Ok(Some(line)) = lines.next_line().await {
		let request: Req = match serde_json::from_str(&line) {
			Ok(r) => r,
			Err(e) => {
				let err_msg = format!("invalid request: {}", e);
				tracing::warn!("{}", err_msg);
				if let Some(ref error_fn) = on_parse_error {
					let resp = error_fn(err_msg);
					if let Ok(mut data) = serde_json::to_vec(&resp) {
						data.push(b'\n');
						if writer.write_all(&data).await.is_err() {
							break;
						}
					}
				}
				continue;
			}
		};

		let response = handler(request).await;

		let mut data = match serde_json::to_vec(&response) {
			Ok(d) => d,
			Err(e) => {
				tracing::error!("failed to serialize response: {}", e);
				continue;
			}
		};
		data.push(b'\n');

		if writer.write_all(&data).await.is_err() {
			break;
		}
	}
}
