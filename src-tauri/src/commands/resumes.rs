use crate::AppState;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResumeItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResumeDetail {
    pub id: String,
    pub name: String,
    pub category: String,
    pub latex_content: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateResumeArgs {
    pub name: String,
    pub category: String,
    pub latex_content: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResumeArgs {
    pub resume_id: String,
    pub name: String,
    pub category: String,
    pub latex_content: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TemplateUsage {
    pub job_id: String,
    pub company_name: String,
    pub job_title: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResumeArgs {
    pub resume_id: String,
}

#[tauri::command]
pub fn check_resume_usage(
    state: State<'_, AppState>,
    resume_id: String,
) -> Result<Vec<TemplateUsage>, String> {
    let mut db_guard = state.db.lock().map_err(|e| format!("Mutex error: {}", e))?;
    let conn = db_guard.as_mut().ok_or("Database connection lost")?;

    let mut stmt = conn
        .prepare(
            "SELECT j.id, j.company_name, j.job_title 
             FROM tailored_resumes tr
             JOIN jobs j ON tr.job_id = j.id
             WHERE tr.base_resume_id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let usage_iter = stmt
        .query_map([&resume_id], |row| {
            Ok(TemplateUsage {
                job_id: row.get(0)?,
                company_name: row.get(1)?,
                job_title: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut usages = Vec::new();
    for usage in usage_iter {
        usages.push(usage.map_err(|e| e.to_string())?);
    }

    Ok(usages)
}

#[tauri::command]
pub fn get_all_resumes(state: State<'_, AppState>) -> Result<Vec<ResumeItem>, String> {
    let mut db_guard = state.db.lock().map_err(|e| format!("Mutex error: {}", e))?;

    if let Some(conn) = db_guard.as_mut() {
        let mut stmt = conn
            .prepare("SELECT id, name, category, created_at, updated_at FROM base_resumes ORDER BY created_at DESC")
            .map_err(|e| format!("Query prepare error: {}", e))?;

        let resumes = stmt
            .query_map([], |row| {
                Ok(ResumeItem {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    category: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })
            .map_err(|e| format!("Query error: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Row collection error: {}", e))?;

        Ok(resumes)
    } else {
        Err("Database connection lost".to_string())
    }
}

#[tauri::command]
pub fn get_resume_by_id(
    state: State<'_, AppState>,
    resume_id: String,
) -> Result<ResumeDetail, String> {
    let mut db_guard = state.db.lock().map_err(|e| format!("Mutex error: {}", e))?;

    if let Some(conn) = db_guard.as_mut() {
        let mut stmt = conn
            .prepare("SELECT id, name, category, latex_content, created_at, updated_at FROM base_resumes WHERE id = ?1")
            .map_err(|e| format!("Query prepare error: {}", e))?;

        let resume = stmt
            .query_row([resume_id], |row| {
                Ok(ResumeDetail {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    category: row.get(2)?,
                    latex_content: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })
            .map_err(|e| format!("Resume not found: {}", e))?;

        Ok(resume)
    } else {
        Err("Database connection lost".to_string())
    }
}

#[tauri::command]
pub fn create_new_resume(
    state: State<'_, AppState>,
    args: CreateResumeArgs,
) -> Result<String, String> {
    let mut db_guard = state.db.lock().map_err(|e| format!("Mutex error: {}", e))?;

    if let Some(conn) = db_guard.as_mut() {
        let resume_id = nanoid!(10);

        conn.execute(
            "INSERT INTO base_resumes (id, name, category, latex_content) VALUES (?1, ?2, ?3, ?4)",
            [&resume_id, &args.name, &args.category, &args.latex_content],
        )
        .map_err(|e| format!("Database error: {}", e))?;

        state.mark_dirty();
        Ok(resume_id)
    } else {
        Err("Database connection lost".to_string())
    }
}

#[tauri::command]
pub fn update_resume(state: State<'_, AppState>, args: UpdateResumeArgs) -> Result<(), String> {
    let mut db_guard = state.db.lock().map_err(|e| format!("Mutex error: {}", e))?;

    if let Some(conn) = db_guard.as_mut() {
        conn.execute(
            "UPDATE base_resumes SET name = ?1, category = ?2, latex_content = ?3, updated_at = CURRENT_TIMESTAMP WHERE id = ?4",
            [&args.name, &args.category, &args.latex_content, &args.resume_id],
        ).map_err(|e| format!("Database error: {}", e))?;

        state.mark_dirty();
        Ok(())
    } else {
        Err("Database connection lost".to_string())
    }
}

#[tauri::command]
pub fn delete_resume(state: State<'_, AppState>, args: DeleteResumeArgs) -> Result<(), String> {
    let mut db_guard = state.db.lock().map_err(|e| format!("Mutex error: {}", e))?;

    if let Some(conn) = db_guard.as_mut() {
        let tx = conn.transaction().map_err(|e| format!("Transaction error: {}", e))?;

        // 1. Delete dependent tailored resumes
        tx.execute(
            "DELETE FROM tailored_resumes WHERE base_resume_id = ?1",
            [&args.resume_id],
        )
        .map_err(|e| format!("Database error (tailored): {}", e))?;

        // 2. Delete the base resume
        tx.execute("DELETE FROM base_resumes WHERE id = ?1", [&args.resume_id])
            .map_err(|e| format!("Database error (base): {}", e))?;

        tx.commit().map_err(|e| format!("Commit error: {}", e))?;

        state.mark_dirty();
        Ok(())
    } else {
        Err("Database connection lost".to_string())
    }
}
