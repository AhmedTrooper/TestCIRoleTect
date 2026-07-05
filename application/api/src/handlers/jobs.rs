use axum::extract::{State, Path};
use axum::{Json, http::StatusCode};
use std::sync::Arc;
use crate::AppState;
use crate::models::{JobPayload, JobParseResult};
use crate::ai;

pub async fn parse_job(
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<JobParseResult>, (StatusCode, String)> {
    let provider = payload["provider"].as_str().ok_or((StatusCode::BAD_REQUEST, "provider missing".to_string()))?;
    let model = payload["model"].as_str().ok_or((StatusCode::BAD_REQUEST, "model missing".to_string()))?;
    let api_key = payload["apiKey"].as_str().ok_or((StatusCode::BAD_REQUEST, "apiKey missing".to_string()))?;
    let raw_jd = payload["rawJd"].as_str().ok_or((StatusCode::BAD_REQUEST, "rawJd missing".to_string()))?;
    let job_url = payload["jobUrl"].as_str();

    ai::parse_job_description(provider, model, api_key, raw_jd, job_url)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))
}

pub async fn save_job(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<JobPayload>,
) -> Result<Json<String>, (StatusCode, String)> {
    let conn = state.db.lock().await;

    conn.execute(
        "INSERT INTO jobs (
            id, company_name, job_title, work_model, employment_type, 
            status, raw_jd, requirements, core_responsibilities,
            custom_instruction, reference_name, 
            reference_email, social_link, job_url,
            base_resume_id, base_cl_id, salary,
            applied_date, interview_date, offer_date, rejected_date, joining_date
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
        rusqlite::params![
            &payload.id,
            &payload.company_name,
            &payload.job_title,
            &payload.work_model,
            &payload.employment_type,
            &payload.status,
            &payload.raw_jd,
            &payload.requirements,
            &payload.core_responsibilities,
            &payload.custom_instruction,
            &payload.reference_name,
            &payload.reference_email,
            &payload.social_link,
            &payload.job_url,
            &payload.base_resume_id,
            &payload.base_cl_id,
            &payload.salary,
            &payload.applied_date,
            &payload.interview_date,
            &payload.offer_date,
            &payload.rejected_date,
            &payload.joining_date,
        ],
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    Ok(Json(payload.id))
}

pub async fn get_all_jobs(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<JobPayload>>, (StatusCode, String)> {
    let conn = state.db.lock().await;
    
    let mut stmt = conn
        .prepare(
            "SELECT id, company_name, job_title, work_model, employment_type, 
                status, raw_jd, requirements, core_responsibilities,
                custom_instruction, reference_name, 
                reference_email, social_link, job_url,
                base_resume_id, base_cl_id,
                salary, applied_date, interview_date, offer_date, rejected_date, joining_date,
                created_at, updated_at
         FROM jobs ORDER BY created_at DESC",
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let job_iter = stmt
        .query_map([], |row| {
            Ok(JobPayload {
                id: row.get(0)?,
                company_name: row.get(1)?,
                job_title: row.get(2)?,
                work_model: row.get(3)?,
                employment_type: row.get(4)?,
                status: row.get(5)?,
                raw_jd: row.get(6)?,
                requirements: row.get(7)?,
                core_responsibilities: row.get(8)?,
                custom_instruction: row.get(9)?,
                reference_name: row.get(10)?,
                reference_email: row.get(11)?,
                social_link: row.get(12)?,
                job_url: row.get(13)?,
                base_resume_id: row.get::<_, Option<String>>(14)?,
                base_cl_id: row.get::<_, Option<String>>(15)?,
                salary: row.get(16)?,
                applied_date: row.get(17)?,
                interview_date: row.get(18)?,
                offer_date: row.get(19)?,
                rejected_date: row.get(20)?,
                joining_date: row.get(21)?,
                created_at: Some(row.get(22)?),
                updated_at: Some(row.get(23)?),
            })
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut jobs = Vec::new();
    for job in job_iter {
        jobs.push(job.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?);
    }
    Ok(Json(jobs))
}

pub async fn get_job_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<JobPayload>, (StatusCode, String)> {
    let conn = state.db.lock().await;
    
    let mut stmt = conn
        .prepare(
            "SELECT id, company_name, job_title, work_model, employment_type, 
                status, raw_jd, requirements, core_responsibilities,
                custom_instruction, reference_name, 
                reference_email, social_link, job_url,
                base_resume_id, base_cl_id,
                salary, applied_date, interview_date, offer_date, rejected_date, joining_date,
                created_at, updated_at
         FROM jobs WHERE id = ?1",
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let job = stmt
        .query_row([&id], |row| {
            Ok(JobPayload {
                id: row.get(0)?,
                company_name: row.get(1)?,
                job_title: row.get(2)?,
                work_model: row.get(3)?,
                employment_type: row.get(4)?,
                status: row.get(5)?,
                raw_jd: row.get(6)?,
                requirements: row.get(7)?,
                core_responsibilities: row.get(8)?,
                custom_instruction: row.get(9)?,
                reference_name: row.get(10)?,
                reference_email: row.get(11)?,
                social_link: row.get(12)?,
                job_url: row.get(13)?,
                base_resume_id: row.get::<_, Option<String>>(14)?,
                base_cl_id: row.get::<_, Option<String>>(15)?,
                salary: row.get(16)?,
                applied_date: row.get(17)?,
                interview_date: row.get(18)?,
                offer_date: row.get(19)?,
                rejected_date: row.get(20)?,
                joining_date: row.get(21)?,
                created_at: Some(row.get(22)?),
                updated_at: Some(row.get(23)?),
            })
        })
        .map_err(|_| (StatusCode::NOT_FOUND, "Job not found".to_string()))?;

    Ok(Json(job))
}

pub async fn delete_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut conn = state.db.lock().await;
    
    let tx = conn.transaction().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Transaction error: {}", e)))?;

    tx.execute("DELETE FROM downloads WHERE job_id = ?1", [&id])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error (downloads): {}", e)))?;

    tx.execute("DELETE FROM tailored_cover_letters WHERE job_id = ?1", [&id])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error (tailored_cover_letters): {}", e)))?;

    tx.execute("DELETE FROM tailored_resumes WHERE job_id = ?1", [&id])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error (tailored_resumes): {}", e)))?;

    tx.execute("DELETE FROM jobs WHERE id = ?1", [&id])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error (jobs): {}", e)))?;

    tx.commit().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Commit error: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}
