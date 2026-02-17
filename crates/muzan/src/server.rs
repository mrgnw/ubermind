use std::future::Future;
use std::sync::Arc;
use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

use crate::paths::DaemonPaths;

pub async fn run_socket_server<Req, Resp, F, Fut>(
	paths: &DaemonPaths,
	handler: F,
) where
	Req: DeserializeOwned + Send + 'static,
	Resp: Serialize + Send + 'static,
	F: Fn(Req) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Resp> + Send,
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

	loop {
		let (stream, _) = match listener.accept().await {
			Ok(s) => s,
			Err(e) => {
				tracing::error!("accept error: {}", e);
				continue;
			}
		};

		let handler = Arc::clone(&handler);
		tokio::spawn(async move {
			handle_connection::<Req, Resp, _, _>(stream, handler).await;
		});
	}
}

async fn handle_connection<Req, Resp, F, Fut>(
	stream: tokio::net::UnixStream,
	handler: Arc<F>,
) where
	Req: DeserializeOwned + Send + 'static,
	Resp: Serialize + Send + 'static,
	F: Fn(Req) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Resp> + Send,
{
	let (reader, mut writer) = stream.into_split();
	let mut lines = BufReader::new(reader).lines();

	while let Ok(Some(line)) = lines.next_line().await {
		let request: Req = match serde_json::from_str(&line) {
			Ok(r) => r,
			Err(e) => {
				tracing::warn!("invalid request: {}", e);
				// Can't send a typed error since we don't know Resp's error variant.
				// Just drop the bad request.
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
