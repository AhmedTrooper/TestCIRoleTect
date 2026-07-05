use rusqlite::{Connection, Result};
use std::fs;
use std::path::PathBuf;

pub fn init_db(db_path: Option<PathBuf>) -> Result<Connection> {
    let db_path = db_path.unwrap_or_else(|| PathBuf::from("roletect.db"));
    
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).expect("Failed to create database directory");
        }
    }

    let conn = Connection::open(db_path)?;

    // Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    conn.execute_batch(
        "
        -- 1. App Settings Table
        CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY, 
            value TEXT NOT NULL
        );

        -- 2. Base Resumes Table
        CREATE TABLE IF NOT EXISTS base_resumes (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            category TEXT NOT NULL,
            latex_content TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TRIGGER IF NOT EXISTS update_base_resumes_modtime 
            AFTER UPDATE ON base_resumes 
            BEGIN UPDATE base_resumes SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id; END;

        -- 2b. Base Cover Letters Table
        CREATE TABLE IF NOT EXISTS base_cover_letters (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            category TEXT NOT NULL,
            latex_content TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TRIGGER IF NOT EXISTS update_base_cover_letters_modtime 
            AFTER UPDATE ON base_cover_letters 
            BEGIN UPDATE base_cover_letters SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id; END;

        -- 3. Jobs Table (Flexible Schema)
        CREATE TABLE IF NOT EXISTS jobs (
            id TEXT PRIMARY KEY,
            company_name TEXT NOT NULL,
            job_title TEXT NOT NULL,
            work_model TEXT DEFAULT 'Remote',
            employment_type TEXT DEFAULT 'Full-time',
            status TEXT NOT NULL DEFAULT 'Drafting',
            raw_jd TEXT NOT NULL,
            requirements TEXT,
            core_responsibilities TEXT,
            custom_instruction TEXT,
            reference_name TEXT,
            reference_email TEXT,
            social_link TEXT,
            job_url TEXT,
            base_resume_id TEXT,
            base_cl_id TEXT,
            salary TEXT,
            applied_date TEXT,
            interview_date TEXT,
            offer_date TEXT,
            rejected_date TEXT,
            joining_date TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (base_resume_id) REFERENCES base_resumes(id),
            FOREIGN KEY (base_cl_id) REFERENCES base_cover_letters(id)
        );
        CREATE TRIGGER IF NOT EXISTS update_jobs_modtime 
            AFTER UPDATE ON jobs 
            BEGIN UPDATE jobs SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id; END;

        -- 4. Tailored Resumes Table (Generated Output)
        CREATE TABLE IF NOT EXISTS tailored_resumes (
            id TEXT PRIMARY KEY,
            job_id TEXT NOT NULL,
            base_resume_id TEXT NOT NULL,
            final_latex_content TEXT NOT NULL,
            is_active BOOLEAN DEFAULT 1,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (job_id) REFERENCES jobs(id),
            FOREIGN KEY (base_resume_id) REFERENCES base_resumes(id)
        );
        CREATE TRIGGER IF NOT EXISTS update_tailored_resumes_modtime 
            AFTER UPDATE ON tailored_resumes 
            BEGIN UPDATE tailored_resumes SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id; END;

        -- 4b. Tailored Cover Letters Table (Generated Output)
        CREATE TABLE IF NOT EXISTS tailored_cover_letters (
            id TEXT PRIMARY KEY,
            job_id TEXT NOT NULL,
            base_cl_id TEXT NOT NULL,
            final_latex_content TEXT NOT NULL,
            is_active BOOLEAN DEFAULT 1,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (job_id) REFERENCES jobs(id),
            FOREIGN KEY (base_cl_id) REFERENCES base_cover_letters(id)
        );
        CREATE TRIGGER IF NOT EXISTS update_tailored_cover_letters_modtime 
            AFTER UPDATE ON tailored_cover_letters 
            BEGIN UPDATE tailored_cover_letters SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id; END;

        -- 5. Standalone Compiler State Table
        CREATE TABLE IF NOT EXISTS compiler_state (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            latex_content TEXT NOT NULL,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        -- 6. Downloads Table
        CREATE TABLE IF NOT EXISTS downloads (
            id TEXT PRIMARY KEY,
            filename TEXT NOT NULL,
            download_type TEXT NOT NULL,
            job_id TEXT,
            content_id TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (job_id) REFERENCES jobs(id)
        );

        -- 7. Themes Table
        CREATE TABLE IF NOT EXISTS themes (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            config TEXT NOT NULL,
            is_builtin BOOLEAN DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        -- 8. Inbox Jobs (Unlisted)
        CREATE TABLE IF NOT EXISTS inbox_jobs (
            id TEXT PRIMARY KEY,
            url TEXT,
            raw_description TEXT NOT NULL,
            status TEXT DEFAULT 'Pending', -- Pending, Processed
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        "
    )?;

    // Ensure secret key for extension exists
    let has_secret: i64 = conn.query_row(
        "SELECT COUNT(*) FROM app_settings WHERE key = 'extension_secret'",
        [],
        |row| row.get(0)
    ).unwrap_or(0);

    if has_secret == 0 {
        let secret = nanoid::nanoid!(32);
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES ('extension_secret', ?1)",
            [&secret],
        )?;
    }

    Ok(conn)
}
