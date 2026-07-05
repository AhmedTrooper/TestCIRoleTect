mod ai;
mod db;
mod models;
mod handlers;

use axum::{
    routing::{get, post, delete},
    Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use rusqlite::Connection;
use tower_http::cors::{Any, CorsLayer};
use dotenvy::dotenv;

pub struct AppState {
    pub db: Mutex<Connection>,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    
    let db_path = std::env::var("DATABASE_URL").map(|s| std::path::PathBuf::from(s)).ok();
    let conn = db::init_db(db_path).expect("Failed to initialize database");
    
    let shared_state = Arc::new(AppState {
        db: Mutex::new(conn),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        
        // Inbox Routes
        .route("/inbox", get(handlers::inbox::get_all_inbox_jobs))
        .route("/inbox/ingest", post(handlers::inbox::ingest_job))
        .route("/inbox/{id}", delete(handlers::inbox::delete_inbox_job))
        .route("/inbox/{id}/process", post(handlers::inbox::mark_inbox_job_processed))
        .route("/inbox/config", get(handlers::inbox::get_extension_config))
        .route("/inbox/secret/reset", post(handlers::inbox::reset_extension_secret))

        // Jobs Routes
        .route("/jobs", get(handlers::jobs::get_all_jobs))
        .route("/jobs", post(handlers::jobs::save_job))
        .route("/jobs/{id}", get(handlers::jobs::get_job_by_id))
        .route("/jobs/{id}", delete(handlers::jobs::delete_job))
        .route("/jobs/parse", post(handlers::jobs::parse_job))

        // Resume Routes
        .route("/resumes", get(handlers::resumes::get_all_resumes))
        .route("/resumes", post(handlers::resumes::create_new_resume))
        .route("/resumes/{id}", get(handlers::resumes::get_resume_by_id))
        .route("/resumes/update", post(handlers::resumes::update_resume))
        .route("/resumes/tailor", post(handlers::resumes::tailor_resume))
        .route("/resumes/latest/{job_id}", get(handlers::resumes::get_latest_tailored_resume))
        
        // Cover Letter Routes
        .route("/cover_letters", get(handlers::cover_letters::get_all_cover_letters))
        .route("/cover_letters", post(handlers::cover_letters::create_new_cover_letter))
        .route("/cover_letters/{id}", get(handlers::cover_letters::get_cover_letter_by_id))
        .route("/cover_letters/update", post(handlers::cover_letters::update_cover_letter))
        .route("/cover_letters/tailor", post(handlers::cover_letters::tailor_cover_letter))
        .route("/cover_letters/latest/{job_id}", get(handlers::cover_letters::get_latest_tailored_cover_letter))

        // Downloads Routes
        .route("/downloads", get(handlers::downloads::get_recent_downloads))
        .route("/downloads", post(handlers::downloads::record_download))

        // Settings Routes
        .route("/settings/ai", get(handlers::settings::get_ai_config))
        .route("/settings/ai", post(handlers::settings::save_ai_config))
        .route("/settings/key", get(handlers::settings::get_api_key))

        // PDF Routes
        .route("/pdf/compile", post(handlers::pdf::compile_latex))
        .route("/pdf/refine", post(handlers::pdf::refine_latex_with_ai))

        // Compiler Routes
        .route("/compiler", get(handlers::compiler::get_compiler_state))
        .route("/compiler", post(handlers::compiler::save_compiler_state))

        .layer(cors)
        .with_state(shared_state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    println!("API server listening on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "running",
        "message": "Roletect API is active"
    }))
}
