use axum::extract::{State, Path};
use axum::{Json, http::StatusCode};
use std::sync::Arc;
use crate::AppState;
use crate::models::{ResumeItem, ResumeDetail, TailoredContent};
use crate::ai;
use nanoid::nanoid;

pub async fn get_all_resumes(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ResumeItem>>, (StatusCode, String)> {
    let conn = state.db.lock().await;
    
    let mut stmt = conn
        .prepare("SELECT id, name, category, created_at, updated_at FROM base_resumes ORDER BY updated_at DESC")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let resume_iter = stmt
        .query_map([], |row| {
            Ok(ResumeItem {
                id: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut resumes = Vec::new();
    for resume in resume_iter {
        resumes.push(resume.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?);
    }
    Ok(Json(resumes))
}

pub async fn get_resume_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ResumeDetail>, (StatusCode, String)> {
    let conn = state.db.lock().await;
    
    let mut stmt = conn
        .prepare("SELECT id, name, category, latex_content, created_at, updated_at FROM base_resumes WHERE id = ?1")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let resume = stmt
        .query_row([&id], |row| {
            Ok(ResumeDetail {
                id: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
                latex_content: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|_| (StatusCode::NOT_FOUND, "Resume not found".to_string()))?;

    Ok(Json(resume))
}

pub async fn create_new_resume(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<String>, (StatusCode, String)> {
    let name = payload["name"].as_str().ok_or((StatusCode::BAD_REQUEST, "name missing".to_string()))?;
    let category = payload["category"].as_str().ok_or((StatusCode::BAD_REQUEST, "category missing".to_string()))?;
    let latex_content = payload["latexContent"].as_str().ok_or((StatusCode::BAD_REQUEST, "latexContent missing".to_string()))?;

    let id = nanoid!(10);
    let conn = state.db.lock().await;
    
    conn.execute(
        "INSERT INTO base_resumes (id, name, category, latex_content) VALUES (?1, ?2, ?3, ?4)",
        [&id, name, category, latex_content],
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(id))
}

pub async fn update_resume(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ResumeDetail>,
) -> Result<StatusCode, (StatusCode, String)> {
    let conn = state.db.lock().await;
    
    conn.execute(
        "UPDATE base_resumes SET name = ?1, category = ?2, latex_content = ?3 WHERE id = ?4",
        [&payload.name, &payload.category, &payload.latex_content, &payload.id],
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

pub async fn tailor_resume(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<String>, (StatusCode, String)> {
    println!("Tailoring resume request received");
    let provider = payload["provider"].as_str().ok_or((StatusCode::BAD_REQUEST, "provider missing".to_string()))?;
    let model = payload["model"].as_str().ok_or((StatusCode::BAD_REQUEST, "model missing".to_string()))?;
    let api_key = payload["apiKey"].as_str().ok_or((StatusCode::BAD_REQUEST, "apiKey missing".to_string()))?;
    let job_id = payload["jobId"].as_str().ok_or((StatusCode::BAD_REQUEST, "jobId missing".to_string()))?;
    let base_resume_id = payload["baseResumeId"].as_str().ok_or((StatusCode::BAD_REQUEST, "baseResumeId missing".to_string()))?;
    let custom_instruction = payload["customInstruction"].as_str();

    let (raw_job_content, requirements, core_responsibilities, base_latex) = {
        let conn = state.db.lock().await;
        
        let (raw_job, reqs, resps): (String, Option<String>, Option<String>) = conn
            .query_row(
                "SELECT raw_jd, requirements, core_responsibilities FROM jobs WHERE id = ?1",
                [&job_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            )
            .map_err(|e| {
                eprintln!("Job Query Error: {}", e);
                (StatusCode::NOT_FOUND, format!("Job not found: {}", job_id))
            })?;

        let latex: String = conn
            .query_row("SELECT latex_content FROM base_resumes WHERE id = ?1", [&base_resume_id], |row| row.get(0))
            .map_err(|e| {
                eprintln!("Resume Query Error: {}", e);
                (StatusCode::NOT_FOUND, format!("Base resume not found: {}", base_resume_id))
            })?;

        (raw_job, reqs, resps, latex)
    };

    println!("Starting AI tailoring...");
    let job_context = format!(
        "Job Description: {}\nRequirements: {}\nResponsibilities: {}",
        raw_job_content,
        requirements.unwrap_or_default(),
        core_responsibilities.unwrap_or_default()
    );

    let tailored_latex = ai::tailor_latex_for_job(
        provider,
        model,
        api_key,
        &base_latex,
        &job_context,
        custom_instruction,
    )
    .await
    .map_err(|e| {
        eprintln!("AI Tailoring Error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e)
    })?;

    println!("AI tailoring complete. Saving to DB...");
    let tailored_id = nanoid!(10);
    
    // Explicit block to ensure MutexGuard and Transaction are dropped before returning
    {
        let mut conn = state.db.lock().await;
        let tx = conn.transaction().map_err(|e| {
            eprintln!("Transaction Error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

        tx.execute(
            "INSERT INTO tailored_resumes (id, job_id, base_resume_id, final_latex_content, is_active)
             VALUES (?1, ?2, ?3, ?4, 1)",
            [&tailored_id, job_id, base_resume_id, &tailored_latex],
        ).map_err(|e| {
            eprintln!("Insert Tailored Resume Error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

        tx.execute(
            "UPDATE jobs SET base_resume_id = ?1 WHERE id = ?2",
            [base_resume_id, job_id],
        ).map_err(|e| {
            eprintln!("Update Job Error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

        tx.commit().map_err(|e| {
            eprintln!("Commit Error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;
    }
    
    println!("Resume tailoring complete and saved.");
    Ok(Json(tailored_id))
}

pub async fn get_latest_tailored_resume(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Result<Json<Option<TailoredContent>>, (StatusCode, String)> {
    let conn = state.db.lock().await;
    
    let mut stmt = conn
        .prepare(
            "SELECT id, base_resume_id, final_latex_content FROM tailored_resumes 
         WHERE job_id = ?1 
         ORDER BY created_at DESC LIMIT 1",
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let result = match stmt.query_row([&job_id], |row| {
        Ok(TailoredContent {
            id: row.get(0)?,
            base_template_id: row.get(1)?,
            content: row.get(2)?,
        })
    }) {
        Ok(v) => Some(v),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    };

    Ok(Json(result))
}
