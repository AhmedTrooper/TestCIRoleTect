use axum::{
    body::Body,
    extract::{State, Json, Path},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use crate::AppState;
use nanoid::nanoid;
use tower_http::cors::{Any, CorsLayer};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio_util::io::ReaderStream;
#[derive(Deserialize)]
pub struct ExtensionPayload {
    pub url: Option<String>,
    pub raw_description: String,
    pub secret: String,
}

#[derive(Serialize)]
pub struct ExtensionResponse {
    pub status: String,
    pub message: String,
}

pub struct ServerState {
    pub app_handle: AppHandle,
}

pub async fn start_server(app_handle: AppHandle) {
    let state = Arc::new(ServerState {
        app_handle: app_handle.clone(),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", axum::routing::get(health_check))
        .route("/inbox/ingest", post(ingest_job))
        .route("/static-pdf/{filename}", get(stream_pdf))
        .layer(cors)
        .with_state(state);

    let ports = [14207, 14213, 1420, 14229, 14235, 14266, 14247, 14298, 14259, 14280];

    for port in ports {
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        let listener = tokio::net::TcpListener::bind(addr).await;
        
        match listener {
            Ok(listener) => {
                println!("Extension server listening on http://{}", addr);
                // but we can save it to settings if we want.
                let handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = handle.state::<AppState>().with_db(|conn| {
                        conn.execute(
                            "INSERT INTO app_settings (key, value) VALUES ('active_server_port', ?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                            [&port.to_string()],
                        ).map_err(|e| e.to_string())?;
                        Ok(())
                    }).await;
                });

                if let Err(e) = axum::serve(listener, app).await {
                    eprintln!("Server error: {}", e);
                }
                break;
            }
            Err(_) => {
                eprintln!("Port {} busy, trying next...", port);
                continue;
            }
        }
    }
}

async fn health_check() -> Json<ExtensionResponse> {
    Json(ExtensionResponse {
        status: "running".to_string(),
        message: "Roletect server is healthy and active".to_string(),
    })
}

async fn ingest_job(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<ExtensionPayload>,
) -> (StatusCode, Json<ExtensionResponse>) {
    let app_state = state.app_handle.state::<AppState>();

    // 1. Verify Secret
    let secret_match = app_state.with_db(|conn| {
        let secret: String = conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'extension_secret'",
            [],
            |row| row.get(0)
        ).unwrap_or_default();
        Ok(secret == payload.secret)
    }).await;

    match secret_match {
        Ok(true) => {
            // 2. Save to inbox_jobs
            let save_result = app_state.with_db(|conn| {
                let id = nanoid!(10);
                conn.execute(
                    "INSERT INTO inbox_jobs (id, url, raw_description, status) VALUES (?1, ?2, ?3, 'Pending')",
                    rusqlite::params![&id, &payload.url, &payload.raw_description],
                ).map_err(|e| e.to_string())?;
                Ok(())
            }).await;

            match save_result {
                Ok(_) => {
                    // Trigger a refresh on frontend if we had a notification system
                    (StatusCode::OK, Json(ExtensionResponse {
                        status: "success".to_string(),
                        message: "Job ingested into vault".to_string(),
                    }))
                }
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ExtensionResponse {
                    status: "error".to_string(),
                    message: format!("Database error: {}", e),
                })),
            }
        }
        _ => (StatusCode::UNAUTHORIZED, Json(ExtensionResponse {
            status: "error".to_string(),
            message: "Invalid secret key".to_string(),
        })),
    }
}

async fn stream_pdf(
    Path(filename): Path<String>,
    State(state): State<Arc<ServerState>>,
    headers: HeaderMap,
) -> axum::response::Response {
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") || !filename.ends_with(".pdf") {
        return (StatusCode::BAD_REQUEST, "Invalid filename").into_response();
    }

    let docs_dir = state.app_handle.path().document_dir().expect("Failed to locate Documents folder");
    let file_path = docs_dir.join("RoleTect").join(filename);

    let mut file = match File::open(&file_path).await {
        Ok(f) => f,
        Err(_) => return (StatusCode::NOT_FOUND, "Output PDF not generated yet").into_response(),
    };

    let file_len = match file.metadata().await {
        Ok(m) => m.len(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Metadata error").into_response(),
    };

    let range_header = headers.get(header::RANGE).and_then(|v| v.to_str().ok());

    if let Some(range_str) = range_header {
        if let Some(bytes_range) = range_str.strip_prefix("bytes=") {
            let parts: Vec<&str> = bytes_range.split('-').collect();
            if parts.len() == 2 {
                let start = parts[0].parse::<u64>().unwrap_or(0);
                let end_str = parts[1];
                let mut end = if end_str.is_empty() { file_len - 1 } else { end_str.parse::<u64>().unwrap_or(file_len - 1) };
                end = std::cmp::min(end, file_len - 1);

                if start <= end {
                    let length = end - start + 1;
                    
                    if file.seek(SeekFrom::Start(start)).await.is_ok() {
                        let limited_stream = file.take(length);
                        let body = Body::from_stream(ReaderStream::new(limited_stream));

                        return (
                            StatusCode::PARTIAL_CONTENT,
                            [
                                (header::CONTENT_TYPE, "application/pdf".to_string()),
                                (header::ACCEPT_RANGES, "bytes".to_string()),
                                (header::CONTENT_RANGE, format!("bytes {}-{}/{}", start, end, file_len)),
                                (header::CONTENT_LENGTH, length.to_string()),
                            ],
                            body,
                        ).into_response();
                    }
                }
            }
        }
    }

    let body = Body::from_stream(ReaderStream::new(file));
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (header::ACCEPT_RANGES, "bytes".to_string()),
            (header::CONTENT_LENGTH, file_len.to_string()),
        ],
        body,
    ).into_response()
}
