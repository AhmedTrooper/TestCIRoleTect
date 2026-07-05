use crate::commands::cover_letters::CoverLetterDetail;
use crate::commands::downloads::DownloadRecord;
use crate::commands::inbox::InboxJob;
use crate::commands::jobs::JobPayload;
use crate::commands::resumes::ResumeDetail;
use crate::AppState;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

/// Keys that are sensitive (secrets, credentials) or per-installation
/// runtime values that must never be exported or overwritten by imports.
const SENSITIVE_EXACT_KEYS: &[&str] = &[
    "extension_secret",
    "active_server_port",
    "ai_provider",
    "ai_model",
];

/// Prefix patterns — any key starting with one of these is sensitive.
const SENSITIVE_PREFIXES: &[&str] = &[
    "s3_",
    "aws_",
    "cloud_",
];

/// Substring patterns — any key containing one of these is sensitive.
const SENSITIVE_SUBSTRINGS: &[&str] = &[
    "api_key",
    "secret",
    "token",
    "password",
    "credential",
    "bucket",
    "custom_base_url",
    "custom_model",
];

pub fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_lowercase();

    if SENSITIVE_EXACT_KEYS.iter().any(|k| lower == *k) {
        return true;
    }

    if SENSITIVE_PREFIXES.iter().any(|p| lower.starts_with(p)) {
        return true;
    }

    if SENSITIVE_SUBSTRINGS.iter().any(|s| lower.contains(s)) {
        return true;
    }

    false
}

/// Snapshot all sensitive settings from the database.
fn snapshot_sensitive_settings(conn: &Connection) -> Vec<(String, String)> {
    let mut stmt = conn
        .prepare("SELECT key, value FROM app_settings")
        .unwrap_or_else(|_| panic!("Failed to prepare snapshot query"));

    stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })
    .unwrap_or_else(|_| panic!("Failed to query settings"))
    .filter_map(|r| r.ok())
    .filter(|(k, _)| is_sensitive_key(k))
    .collect()
}

/// Restore previously-snapshotted sensitive settings back into the database.
fn restore_sensitive_settings(conn: &Connection, snapshot: &[(String, String)]) {
    for (key, value) in snapshot {
        let _ = conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [key, value],
        );
    }
}

#[derive(Serialize, Deserialize)]
pub struct TailoredResumeExport {
    pub id: String,
    pub job_id: String,
    pub base_resume_id: String,
    pub final_latex_content: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct TailoredCoverLetterExport {
    pub id: String,
    pub job_id: String,
    pub base_cl_id: String,
    pub final_latex_content: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct ThemeExport {
    pub id: String,
    pub name: String,
    pub config: String,
    pub is_builtin: bool,
    pub created_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct SettingExport {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct AppDataExport {
    pub jobs: Vec<JobPayload>,
    pub base_resumes: Vec<ResumeDetail>,
    pub base_cover_letters: Vec<CoverLetterDetail>,
    pub tailored_resumes: Vec<TailoredResumeExport>,
    pub tailored_cover_letters: Vec<TailoredCoverLetterExport>,
    pub downloads: Vec<DownloadRecord>,
    pub themes: Vec<ThemeExport>,
    pub app_settings: Vec<SettingExport>,
    pub inbox_jobs: Vec<InboxJob>,
    pub compiler_state: Option<String>,
    pub exported_at: String,
}

#[tauri::command]
pub fn export_all_data_core(state: &AppState) -> Result<AppDataExport, String> {
    let mut db_guard = state.db.lock().map_err(|e| format!("Mutex error: {}", e))?;
    let conn = db_guard.as_mut().ok_or("Database connection lost")?;

    // 1. Fetch Jobs
    let mut stmt = conn
        .prepare(
            "SELECT id, company_name, job_title, work_model, employment_type, 
                status, raw_jd, requirements, core_responsibilities,
                custom_instruction, reference_name, 
                reference_email, social_link, job_url,
                base_resume_id, base_cl_id,
                salary, applied_date, interview_date, offer_date, rejected_date, joining_date,
                created_at, updated_at
         FROM jobs",
        )
        .map_err(|e| e.to_string())?;

    let jobs = stmt
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
                base_resume_id: row.get(14)?,
                base_cl_id: row.get(15)?,
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
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // 2. Fetch Base Resumes
    let mut stmt = conn
        .prepare(
            "SELECT id, name, category, latex_content, created_at, updated_at FROM base_resumes",
        )
        .map_err(|e| e.to_string())?;

    let base_resumes = stmt
        .query_map([], |row| {
            Ok(ResumeDetail {
                id: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
                latex_content: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // 2b. Fetch Base Cover Letters
    let mut stmt = conn.prepare(
        "SELECT id, name, category, latex_content, created_at, updated_at FROM base_cover_letters"
    ).map_err(|e| e.to_string())?;

    let base_cover_letters = stmt
        .query_map([], |row| {
            Ok(CoverLetterDetail {
                id: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
                latex_content: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // 3. Fetch Tailored Resumes
    let mut stmt = conn.prepare(
        "SELECT id, job_id, base_resume_id, final_latex_content, is_active, created_at, updated_at 
         FROM tailored_resumes"
    ).map_err(|e| e.to_string())?;

    let tailored_resumes = stmt
        .query_map([], |row| {
            Ok(TailoredResumeExport {
                id: row.get(0)?,
                job_id: row.get(1)?,
                base_resume_id: row.get(2)?,
                final_latex_content: row.get(3)?,
                is_active: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // 3b. Fetch Tailored Cover Letters
    let mut stmt = conn
        .prepare(
            "SELECT id, job_id, base_cl_id, final_latex_content, is_active, created_at, updated_at 
         FROM tailored_cover_letters",
        )
        .map_err(|e| e.to_string())?;

    let tailored_cover_letters = stmt
        .query_map([], |row| {
            Ok(TailoredCoverLetterExport {
                id: row.get(0)?,
                job_id: row.get(1)?,
                base_cl_id: row.get(2)?,
                final_latex_content: row.get(3)?,
                is_active: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // 4. Fetch Downloads
    let mut stmt = conn
        .prepare(
            "SELECT id, filename, download_type, job_id, content_id, created_at FROM downloads",
        )
        .map_err(|e| e.to_string())?;

    let downloads = stmt
        .query_map([], |row| {
            Ok(DownloadRecord {
                id: row.get(0)?,
                filename: row.get(1)?,
                download_type: row.get(2)?,
                job_id: row.get(3)?,
                content_id: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // 5. Fetch Themes
    let mut stmt = conn
        .prepare("SELECT id, name, config, is_builtin, created_at FROM themes")
        .map_err(|e| e.to_string())?;

    let themes = stmt
        .query_map([], |row| {
            Ok(ThemeExport {
                id: row.get(0)?,
                name: row.get(1)?,
                config: row.get(2)?,
                is_builtin: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // 6. Fetch App Settings (excluding sensitive keys)
    let mut stmt = conn
        .prepare("SELECT key, value FROM app_settings")
        .map_err(|e| e.to_string())?;

    let app_settings: Vec<SettingExport> = stmt
        .query_map([], |row| {
            Ok(SettingExport {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .filter(|s| !is_sensitive_key(&s.key))
        .collect();

    // 7. Fetch Inbox Jobs
    let mut stmt = conn
        .prepare("SELECT id, url, raw_description, status, created_at FROM inbox_jobs")
        .map_err(|e| e.to_string())?;

    let inbox_jobs = stmt
        .query_map([], |row| {
            Ok(InboxJob {
                id: row.get(0)?,
                url: row.get(1)?,
                raw_description: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // 8. Fetch Compiler State
    let compiler_state: Option<String> = conn
        .query_row(
            "SELECT latex_content FROM compiler_state WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();

    Ok(AppDataExport {
        jobs,
        base_resumes,
        base_cover_letters,
        tailored_resumes,
        tailored_cover_letters,
        downloads,
        themes,
        app_settings,
        inbox_jobs,
        compiler_state,
        exported_at: chrono::Local::now().to_rfc3339(),
    })
}

#[tauri::command]
pub async fn export_all_data(state: State<'_, AppState>) -> Result<AppDataExport, String> {
    export_all_data_core(&state)
}

pub fn import_data_core(state: &AppState, data: AppDataExport, mode: String) -> Result<(), String> {
    let mut db_guard = state.db.lock().map_err(|e| format!("Mutex error: {}", e))?;
    let conn = db_guard.as_mut().ok_or("Database connection lost")?;

    // Snapshot ALL sensitive settings BEFORE any mutations so they survive import.
    let sensitive_snapshot = snapshot_sensitive_settings(conn);

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    if mode == "overwrite" {
        // Clear everything - order matters because of foreign keys
        tx.execute("DELETE FROM downloads", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM tailored_cover_letters", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM tailored_resumes", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM jobs", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM base_cover_letters", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM base_resumes", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM compiler_state", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM themes WHERE is_builtin = 0", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM inbox_jobs", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM app_settings", [])
            .map_err(|e| e.to_string())?;
    }

    // 1. Import Base Resumes
    for resume in data.base_resumes {
        tx.execute(
            "INSERT INTO base_resumes (id, name, category, latex_content, created_at, updated_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET 
                name=excluded.name, 
                category=excluded.category, 
                latex_content=excluded.latex_content,
                updated_at=excluded.updated_at",
            (
                &resume.id,
                &resume.name,
                &resume.category,
                &resume.latex_content,
                &resume.created_at,
                &resume.updated_at,
            ),
        )
        .map_err(|e| e.to_string())?;
    }

    // 1b. Import Base Cover Letters
    for cl in data.base_cover_letters {
        tx.execute(
            "INSERT INTO base_cover_letters (id, name, category, latex_content, created_at, updated_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET 
                name=excluded.name, 
                category=excluded.category, 
                latex_content=excluded.latex_content,
                updated_at=excluded.updated_at",
            (
                &cl.id,
                &cl.name,
                &cl.category,
                &cl.latex_content,
                &cl.created_at,
                &cl.updated_at,
            ),
        ).map_err(|e| e.to_string())?;
    }

    // 2. Import Jobs
    for job in data.jobs {
        tx.execute(
            "INSERT INTO jobs (
                id, company_name, job_title, work_model, employment_type, 
                status, raw_jd, requirements, core_responsibilities,
                custom_instruction, reference_name, 
                reference_email, social_link, job_url,
                base_resume_id, base_cl_id,
                salary, applied_date, interview_date, offer_date, rejected_date, joining_date,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)
            ON CONFLICT(id) DO UPDATE SET 
                company_name=excluded.company_name,
                job_title=excluded.job_title,
                work_model=excluded.work_model,
                employment_type=excluded.employment_type,
                status=excluded.status,
                raw_jd=excluded.raw_jd,
                requirements=excluded.requirements,
                core_responsibilities=excluded.core_responsibilities,
                custom_instruction=excluded.custom_instruction,
                reference_name=excluded.reference_name,
                reference_email=excluded.reference_email,
                social_link=excluded.social_link,
                job_url=excluded.job_url,
                base_resume_id=excluded.base_resume_id,
                base_cl_id=excluded.base_cl_id,
                salary=excluded.salary,
                applied_date=excluded.applied_date,
                interview_date=excluded.interview_date,
                offer_date=excluded.offer_date,
                rejected_date=excluded.rejected_date,
                joining_date=excluded.joining_date,
                updated_at=excluded.updated_at",
            params![
                &job.id,
                &job.company_name,
                &job.job_title,
                &job.work_model,
                &job.employment_type,
                &job.status,
                &job.raw_jd,
                &job.requirements,
                &job.core_responsibilities,
                &job.custom_instruction,
                &job.reference_name,
                &job.reference_email,
                &job.social_link,
                &job.job_url,
                &job.base_resume_id,
                &job.base_cl_id,
                &job.salary,
                &job.applied_date,
                &job.interview_date,
                &job.offer_date,
                &job.rejected_date,
                &job.joining_date,
                &job.created_at
                    .unwrap_or_else(|| chrono::Local::now().to_rfc3339()),
                &job.updated_at
                    .unwrap_or_else(|| chrono::Local::now().to_rfc3339()),
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    // 3. Import Tailored Resumes
    for tailored in data.tailored_resumes {
        tx.execute(
            "INSERT INTO tailored_resumes (id, job_id, base_resume_id, final_latex_content, is_active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET 
                final_latex_content=excluded.final_latex_content,
                is_active=excluded.is_active,
                updated_at=excluded.updated_at",
            (
                &tailored.id,
                &tailored.job_id,
                &tailored.base_resume_id,
                &tailored.final_latex_content,
                &tailored.is_active,
                &tailored.created_at,
                &tailored.updated_at,
            ),
        ).map_err(|e| e.to_string())?;
    }

    // 3b. Import Tailored Cover Letters
    for tailored in data.tailored_cover_letters {
        tx.execute(
            "INSERT INTO tailored_cover_letters (id, job_id, base_cl_id, final_latex_content, is_active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET 
                final_latex_content=excluded.final_latex_content,
                is_active=excluded.is_active,
                updated_at=excluded.updated_at",
            (
                &tailored.id,
                &tailored.job_id,
                &tailored.base_cl_id,
                &tailored.final_latex_content,
                &tailored.is_active,
                &tailored.created_at,
                &tailored.updated_at,
            ),
        ).map_err(|e| e.to_string())?;
    }

    // 4. Import Downloads
    for download in data.downloads {
        tx.execute(
            "INSERT INTO downloads (id, filename, download_type, job_id, content_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO NOTHING",
            (
                &download.id,
                &download.filename,
                &download.download_type,
                &download.job_id,
                &download.content_id,
                &download.created_at,
            ),
        )
        .map_err(|e| e.to_string())?;
    }

    // 5. Import Themes
    for theme in data.themes {
        // Skip built-in themes as they are re-populated on app start
        if theme.is_builtin { continue; }
        
        tx.execute(
            "INSERT INTO themes (id, name, config, is_builtin, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET 
                name=excluded.name,
                config=excluded.config,
                is_builtin=excluded.is_builtin",
            (
                &theme.id,
                &theme.name,
                &theme.config,
                &theme.is_builtin,
                &theme.created_at,
            ),
        )
        .map_err(|e| e.to_string())?;
    }

    // 6. Import App Settings (skip any sensitive keys that leaked into the file)
    for setting in data.app_settings {
        if is_sensitive_key(&setting.key) {
            continue;
        }
        tx.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            (&setting.key, &setting.value),
        )
        .map_err(|e| e.to_string())?;
    }

    // 7. Import Compiler State
    if let Some(content) = data.compiler_state {
        tx.execute(
            "INSERT INTO compiler_state (id, latex_content) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET latex_content=excluded.latex_content",
            [&content],
        )
        .map_err(|e| e.to_string())?;
    }

    // 8. Import Inbox Jobs
    for job in data.inbox_jobs {
        tx.execute(
            "INSERT INTO inbox_jobs (id, url, raw_description, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET 
                url=excluded.url,
                raw_description=excluded.raw_description,
                status=excluded.status",
            (
                &job.id,
                &job.url,
                &job.raw_description,
                &job.status,
                &job.created_at,
            ),
        )
        .map_err(|e| e.to_string())?;
    }

    // Restore sensitive settings that were snapshotted before the import.
    // This runs inside the transaction so it's atomic.
    restore_sensitive_settings(&tx, &sensitive_snapshot);

    tx.commit().map_err(|e| e.to_string())?;
    
    state.mark_dirty();
    Ok(())
}

#[tauri::command]
pub async fn import_data(
    state: State<'_, AppState>,
    data: AppDataExport,
    mode: String,
) -> Result<(), String> {
    import_data_core(&state, data, mode)
}

#[tauri::command]
pub async fn auto_local_backup(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    use tauri::Manager;
    
    let data = export_all_data_core(&state)?;
    let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
    
    let docs_dir = app.path().document_dir().map_err(|_| "Could not find documents directory".to_string())?;
    let backup_dir = docs_dir.join("RoleTect-Backups");
    
    if !backup_dir.exists() {
        std::fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    }
    
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M");
    let file_path = backup_dir.join(format!("RoleTect_Backup_{}.json", timestamp));
    
    std::fs::write(&file_path, json).map_err(|e| format!("Failed to write local backup: {}", e))?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_sensitive_key tests ---

    #[test]
    fn exact_keys_are_sensitive() {
        assert!(is_sensitive_key("extension_secret"));
        assert!(is_sensitive_key("active_server_port"));
        assert!(is_sensitive_key("ai_provider"));
        assert!(is_sensitive_key("ai_model"));
    }

    #[test]
    fn exact_keys_are_case_insensitive() {
        assert!(is_sensitive_key("Extension_Secret"));
        assert!(is_sensitive_key("AI_PROVIDER"));
        assert!(is_sensitive_key("AI_MODEL"));
    }

    #[test]
    fn prefix_keys_are_sensitive() {
        assert!(is_sensitive_key("s3_bucket_name"));
        assert!(is_sensitive_key("s3_region"));
        assert!(is_sensitive_key("aws_access_key_id"));
        assert!(is_sensitive_key("aws_secret_access_key"));
        assert!(is_sensitive_key("cloud_backup_url"));
    }

    #[test]
    fn substring_keys_are_sensitive() {
        assert!(is_sensitive_key("gemini_custom_base_url"));
        assert!(is_sensitive_key("openai_custom_base_url"));
        assert!(is_sensitive_key("anthropic_custom_model"));
        assert!(is_sensitive_key("ollama_custom_base_url"));
        assert!(is_sensitive_key("some_api_key_for_thing"));
        assert!(is_sensitive_key("my_secret_value"));
        assert!(is_sensitive_key("auth_token"));
        assert!(is_sensitive_key("db_password"));
        assert!(is_sensitive_key("bedrock_credential"));
        assert!(is_sensitive_key("backup_bucket"));
    }

    #[test]
    fn non_sensitive_keys_pass_through() {
        assert!(!is_sensitive_key("active_theme"));
        assert!(!is_sensitive_key("latex_workspace"));
        assert!(!is_sensitive_key("last_opened_file"));
        assert!(!is_sensitive_key("font_family"));
        assert!(!is_sensitive_key("font_size"));
        assert!(!is_sensitive_key("font_weight"));
        assert!(!is_sensitive_key("font_style"));
        assert!(!is_sensitive_key("auto_compile"));
        assert!(!is_sensitive_key("diagram_workspace"));
        assert!(!is_sensitive_key("last_opened_diagram"));
    }

    // --- snapshot / restore integration tests ---

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("
            CREATE TABLE app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
        ").unwrap();
        conn
    }

    #[test]
    fn snapshot_captures_only_sensitive_keys() {
        let conn = setup_test_db();
        conn.execute("INSERT INTO app_settings VALUES ('ai_provider', 'gemini')", []).unwrap();
        conn.execute("INSERT INTO app_settings VALUES ('ai_model', 'gemini-2.5-pro')", []).unwrap();
        conn.execute("INSERT INTO app_settings VALUES ('extension_secret', 'abc123')", []).unwrap();
        conn.execute("INSERT INTO app_settings VALUES ('active_theme', 'dracula')", []).unwrap();
        conn.execute("INSERT INTO app_settings VALUES ('font_size', '14')", []).unwrap();
        conn.execute("INSERT INTO app_settings VALUES ('s3_bucket_name', 'my-bucket')", []).unwrap();

        let snapshot = snapshot_sensitive_settings(&conn);
        let keys: Vec<&str> = snapshot.iter().map(|(k, _)| k.as_str()).collect();

        assert!(keys.contains(&"ai_provider"));
        assert!(keys.contains(&"ai_model"));
        assert!(keys.contains(&"extension_secret"));
        assert!(keys.contains(&"s3_bucket_name"));
        assert!(!keys.contains(&"active_theme"));
        assert!(!keys.contains(&"font_size"));
    }

    #[test]
    fn restore_brings_back_sensitive_keys_after_wipe() {
        let conn = setup_test_db();
        conn.execute("INSERT INTO app_settings VALUES ('ai_provider', 'gemini')", []).unwrap();
        conn.execute("INSERT INTO app_settings VALUES ('extension_secret', 'my-secret-123')", []).unwrap();
        conn.execute("INSERT INTO app_settings VALUES ('active_theme', 'dracula')", []).unwrap();

        let snapshot = snapshot_sensitive_settings(&conn);

        // Wipe everything (simulating overwrite mode)
        conn.execute("DELETE FROM app_settings", []).unwrap();

        // Import some foreign settings
        conn.execute("INSERT INTO app_settings VALUES ('active_theme', 'nord-dark')", []).unwrap();
        conn.execute("INSERT INTO app_settings VALUES ('font_size', '16')", []).unwrap();

        // Restore sensitive keys
        restore_sensitive_settings(&conn, &snapshot);

        // Sensitive keys restored
        let provider: String = conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'ai_provider'", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(provider, "gemini");

        let secret: String = conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'extension_secret'", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(secret, "my-secret-123");

        // Non-sensitive keys from import are untouched
        let theme: String = conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'active_theme'", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(theme, "nord-dark");

        let font: String = conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'font_size'", [], |r| r.get(0)
        ).unwrap();
        assert_eq!(font, "16");
    }

    #[test]
    fn import_skips_sensitive_keys_from_incoming_data() {
        // Verify the guard: even if a backup file somehow contains sensitive keys,
        // they should be skipped during import.
        let incoming = vec![
            SettingExport { key: "active_theme".to_string(), value: "monokai".to_string() },
            SettingExport { key: "ai_provider".to_string(), value: "openai".to_string() },
            SettingExport { key: "extension_secret".to_string(), value: "LEAKED".to_string() },
            SettingExport { key: "s3_bucket_name".to_string(), value: "evil-bucket".to_string() },
            SettingExport { key: "font_family".to_string(), value: "Inter".to_string() },
        ];

        let safe: Vec<&SettingExport> = incoming.iter()
            .filter(|s| !is_sensitive_key(&s.key))
            .collect();

        assert_eq!(safe.len(), 2);
        assert_eq!(safe[0].key, "active_theme");
        assert_eq!(safe[1].key, "font_family");
    }
}
