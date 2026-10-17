//! Avvio del server: runtime tokio su un thread proprio, router axum.

use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use axum::routing::get;
use axum::{Extension, Json, Router};
use h6022_core::EngineHandle;
use h6022_record::Recorder;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio_util::sync::CancellationToken;

use crate::tools::H6022Mcp;
use crate::{AppState, ServerConfig, ws};

pub struct ServerHandle {
    addr: SocketAddr,
    cancel: CancellationToken,
    thread: Option<JoinHandle<()>>,
    recorder: Arc<Mutex<Option<Recorder>>>,
}

/// Fa il bind su `config.addr` prima di restituire: un errore di porta occupata arriva qui.
/// `source` è "hardware" o "simulator".
pub fn spawn(engine: EngineHandle, source: &'static str, config: ServerConfig) -> anyhow::Result<ServerHandle> {
    let listener = StdTcpListener::bind(config.addr)?;
    listener.set_nonblocking(true)?;
    let addr = listener.local_addr()?;

    let recorder = Arc::new(Mutex::new(None));
    let state = AppState { engine, source, addr, recorder: Arc::clone(&recorder) };
    let cancel = CancellationToken::new();
    let token = cancel.clone();

    let thread = thread::Builder::new().name("h6022-server".into()).spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("runtime tokio del server");
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(listener).expect("listener tokio");
            let app = router(state, token.clone());
            if let Err(e) = axum::serve(listener, app).with_graceful_shutdown(token.cancelled_owned()).await {
                eprintln!("server HTTP interrotto: {e}");
            }
        });
    })?;

    Ok(ServerHandle { addr, cancel, thread: Some(thread), recorder })
}

fn router(state: AppState, cancel: CancellationToken) -> Router {
    let mcp_state = state.clone();
    let mcp = StreamableHttpService::new(
        move || Ok(H6022Mcp::new(mcp_state.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_cancellation_token(cancel.child_token()),
    );
    Router::new()
        .nest_service("/mcp", mcp)
        .route("/ws", get(ws::upgrade))
        .route("/health", get(health))
        .layer(Extension(cancel))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") }))
}

impl ServerHandle {
    /// Indirizzo effettivo (con porta 0 nel config, quella assegnata dal sistema).
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Ferma il server e la registrazione in corso, poi attende il thread.
    pub fn shutdown(mut self) {
        self.cancel.cancel();
        self.join();
        let recording = self.recorder.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(recording) = recording {
            let _ = recording.stop();
        }
    }

    /// Blocca finché il server non termina.
    pub fn wait(mut self) {
        self.join();
    }

    fn join(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
