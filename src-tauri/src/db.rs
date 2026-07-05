use rusqlite::{Connection, Result};
use std::fs;
use tauri::{AppHandle, Manager};

pub fn init_db(app: &AppHandle) -> Result<Connection> {
    let app_dir = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data dir");
    fs::create_dir_all(&app_dir).expect("Failed to create app data dir");
    let db_path = app_dir.join("roletect.db");

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

    // --- MIGRATIONS ---

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

    // 0. Ensure 'name' is unique in themes table (for existing databases)
    let table_info: String = conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='themes'",
        [],
        |row| row.get(0)
    ).unwrap_or_default();

    if !table_info.contains("UNIQUE") {
        println!("Migrating 'themes' table to ensure unique names...");
        conn.execute_batch("
            PRAGMA foreign_keys=OFF;
            BEGIN TRANSACTION;
            CREATE TABLE themes_new (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                config TEXT NOT NULL,
                is_builtin BOOLEAN DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            INSERT OR IGNORE INTO themes_new SELECT * FROM themes;
            DROP TABLE themes;
            ALTER TABLE themes_new RENAME TO themes;
            COMMIT;
            PRAGMA foreign_keys=ON;
        ")?;
    }

    // --- BUILT-IN THEMES REFRESH ---
    let builtin_themes = vec![
        // --- 1. CORE FAVORITES ---
        ("github-dark", "GitHub Dark", r##" {
            "--bg": "#0d1117", "--bg-accent": "#161b22", "--surface": "#21262d", "--surface-soft": "#30363d",
            "--ink": "#c9d1d9", "--muted": "#8b949e", "--line": "#30363d", "--accent": "#238636",
            "--accent-soft": "rgba(35, 134, 54, 0.15)", "--warning": "#f85149"
        } "##),
        ("github-light", "GitHub Light", r##" {
            "--bg": "#ffffff", "--bg-accent": "#f6f8fa", "--surface": "#ffffff", "--surface-soft": "#f6f8fa",
            "--ink": "#24292f", "--muted": "#57606a", "--line": "#d0d7de", "--accent": "#0969da",
            "--accent-soft": "rgba(9, 105, 218, 0.1)", "--warning": "#cf222e"
        } "##),

        // --- 2. ELITE DARK COLLECTION ---
        ("dracula", "Dracula", r##" {
            "--bg": "#282a36", "--bg-accent": "#1e1f29", "--surface": "#343746", "--surface-soft": "#44475a",
            "--ink": "#f8f8f2", "--muted": "#6272a4", "--line": "#44475a", "--accent": "#bd93f9",
            "--accent-soft": "rgba(189, 147, 249, 0.15)", "--warning": "#ff5555"
        } "##),
        ("nord-dark", "Nord Dark", r##" {
            "--bg": "#2e3440", "--bg-accent": "#242933", "--surface": "#3b4252", "--surface-soft": "#434c5e",
            "--ink": "#eceff4", "--muted": "#4c566a", "--line": "#3b4252", "--accent": "#88c0d0",
            "--accent-soft": "rgba(136, 192, 208, 0.15)", "--warning": "#bf616a"
        } "##),
        ("one-dark", "One Dark Pro", r##" {
            "--bg": "#282c34", "--bg-accent": "#21252b", "--surface": "#2c313a", "--surface-soft": "#3e4451",
            "--ink": "#abb2bf", "--muted": "#5c6370", "--line": "#3e4451", "--accent": "#61afef",
            "--accent-soft": "rgba(97, 175, 239, 0.15)", "--warning": "#e06c75"
        } "##),
        ("catppuccin-mocha", "Catppuccin Mocha", r##" {
            "--bg": "#1e1e2e", "--bg-accent": "#181825", "--surface": "#313244", "--surface-soft": "#45475a",
            "--ink": "#cdd6f4", "--muted": "#7f849c", "--line": "#45475a", "--accent": "#89b4fa",
            "--accent-soft": "rgba(137, 180, 250, 0.15)", "--warning": "#f38ba8"
        } "##),
        ("tokyo-night-storm", "Tokyo Night Storm", r##" {
            "--bg": "#24283b", "--bg-accent": "#1a1b26", "--surface": "#2f354a", "--surface-soft": "#414868",
            "--ink": "#a9b1d6", "--muted": "#565f89", "--line": "#2f354a", "--accent": "#7aa2f7",
            "--accent-soft": "rgba(122, 162, 247, 0.15)", "--warning": "#f7768e"
        } "##),
        ("gruvbox-dark", "Gruvbox Dark", r##" {
            "--bg": "#282828", "--bg-accent": "#1d2021", "--surface": "#3c3836", "--surface-soft": "#504945",
            "--ink": "#ebdbb2", "--muted": "#928374", "--line": "#3c3836", "--accent": "#fabd2f",
            "--accent-soft": "rgba(250, 189, 47, 0.15)", "--warning": "#fb4934"
        } "##),
        ("night-owl", "Night Owl", r##" {
            "--bg": "#011627", "--bg-accent": "#010e1b", "--surface": "#0b2942", "--surface-soft": "#1d3b53",
            "--ink": "#d6deeb", "--muted": "#5f7e97", "--line": "#1d3b53", "--accent": "#82aaff",
            "--accent-soft": "rgba(130, 170, 255, 0.15)", "--warning": "#ef5350"
        } "##),
        ("everforest-dark", "Everforest Dark", r##" {
            "--bg": "#2d353b", "--bg-accent": "#232a2e", "--surface": "#343f44", "--surface-soft": "#3d484d",
            "--ink": "#d3c6aa", "--muted": "#859289", "--line": "#475258", "--accent": "#a7c080",
            "--accent-soft": "rgba(167, 192, 128, 0.15)", "--warning": "#e67e80"
        } "##),
        ("material-ocean", "Material Ocean", r##" {
            "--bg": "#0f111a", "--bg-accent": "#090b10", "--surface": "#1a1c25", "--surface-soft": "#242938",
            "--ink": "#a6accd", "--muted": "#464b5d", "--line": "#242938", "--accent": "#80cbc4",
            "--accent-soft": "rgba(128, 203, 196, 0.15)", "--warning": "#ff5370"
        } "##),
        ("monokai-pro", "Monokai Pro", r##" {
            "--bg": "#2d2a2e", "--bg-accent": "#221f22", "--surface": "#403e41", "--surface-soft": "#5b595c",
            "--ink": "#fcfcfa", "--muted": "#727072", "--line": "#403e41", "--accent": "#ffd866",
            "--accent-soft": "rgba(255, 216, 102, 0.15)", "--warning": "#ff6188"
        } "##),
        ("rose-pine", "Rosé Pine", r##" {
            "--bg": "#191724", "--bg-accent": "#1f1d2e", "--surface": "#26233a", "--surface-soft": "#403d52",
            "--ink": "#e0def4", "--muted": "#908caa", "--line": "#403d52", "--accent": "#ebbcba",
            "--accent-soft": "rgba(235, 188, 186, 0.15)", "--warning": "#eb6f92"
        } "##),
        ("kanagawa-dragon", "Kanagawa Dragon", r##" {
            "--bg": "#181820", "--bg-accent": "#121217", "--surface": "#22222a", "--surface-soft": "#2d2d38",
            "--ink": "#c8c093", "--muted": "#727169", "--line": "#2d2d38", "--accent": "#7e9cd8",
            "--accent-soft": "rgba(126, 156, 216, 0.15)", "--warning": "#e82424"
        } "##),
        ("shades-of-purple", "Shades of Purple", r##" {
            "--bg": "#2d2b55", "--bg-accent": "#222244", "--surface": "#3e3b85", "--surface-soft": "#4d4d8c",
            "--ink": "#ffffff", "--muted": "#a599e9", "--line": "#4d4d8c", "--accent": "#fad000",
            "--accent-soft": "rgba(250, 208, 0, 0.15)", "--warning": "#ff628c"
        } "##),
        ("cyberpunk-dark", "Cyberpunk Dark", r##" {
            "--bg": "#000b1e", "--bg-accent": "#000816", "--surface": "#001b3a", "--surface-soft": "#002a50",
            "--ink": "#00ff9f", "--muted": "#00b8ff", "--line": "#002a50", "--accent": "#f300ff",
            "--accent-soft": "rgba(243, 0, 255, 0.15)", "--warning": "#ff003c"
        } "##),
        ("oceanic-next", "Oceanic Next", r##" {
            "--bg": "#1b2b34", "--bg-accent": "#16252d", "--surface": "#343d46", "--surface-soft": "#4f5b66",
            "--ink": "#d8dee9", "--muted": "#65737e", "--line": "#4f5b66", "--accent": "#6699cc",
            "--accent-soft": "rgba(102, 153, 204, 0.15)", "--warning": "#ec5f67"
        } "##),
        ("cobalt2", "Cobalt2", r##" {
            "--bg": "#193549", "--bg-accent": "#152c3d", "--surface": "#1d405d", "--surface-soft": "#35434d",
            "--ink": "#ffffff", "--muted": "#0088ff", "--line": "#35434d", "--accent": "#ffc600",
            "--accent-soft": "rgba(255, 198, 0, 0.15)", "--warning": "#ff0000"
        } "##),
        ("synthwave-84", "SynthWave '84", r##" {
            "--bg": "#262335", "--bg-accent": "#241b2f", "--surface": "#34294f", "--surface-soft": "#463465",
            "--ink": "#ffffff", "--muted": "#848bbd", "--line": "#463465", "--accent": "#ff7edb",
            "--accent-soft": "rgba(255, 126, 219, 0.15)", "--warning": "#fe4450"
        } "##),
        ("horizon-dark", "Horizon Dark", r##" {
            "--bg": "#1c1e26", "--bg-accent": "#16161c", "--surface": "#232530", "--surface-soft": "#2e303e",
            "--ink": "#d5d8da", "--muted": "#6c6f93", "--line": "#2e303e", "--accent": "#e95678",
            "--accent-soft": "rgba(233, 86, 120, 0.15)", "--warning": "#fab795"
        } "##),
        ("ayu-dark", "Ayu Dark", r##" {
            "--bg": "#0a0e14", "--bg-accent": "#01060e", "--surface": "#11151c", "--surface-soft": "#171b24",
            "--ink": "#b3b1ad", "--muted": "#475258", "--line": "#171b24", "--accent": "#e6b450",
            "--accent-soft": "rgba(230, 180, 80, 0.15)", "--warning": "#ff3333"
        } "##),
        ("jellybeans", "Jellybeans", r##" {
            "--bg": "#151515", "--bg-accent": "#101010", "--surface": "#252525", "--surface-soft": "#303030",
            "--ink": "#e8e8d3", "--muted": "#888888", "--line": "#252525", "--accent": "#8fbfdc",
            "--accent-soft": "rgba(143, 191, 220, 0.15)", "--warning": "#cf6a4c"
        } "##),
        ("iceberg-dark", "Iceberg Dark", r##" {
            "--bg": "#161821", "--bg-accent": "#0f1117", "--surface": "#1e2132", "--surface-soft": "#2d3143",
            "--ink": "#c6c8d1", "--muted": "#6b7089", "--line": "#2d3143", "--accent": "#84a0c6",
            "--accent-soft": "rgba(132, 160, 198, 0.15)", "--warning": "#e27878"
        } "##),
        ("panda-dark", "Panda Dark", r##" {
            "--bg": "#292a2b", "--bg-accent": "#212122", "--surface": "#343536", "--surface-soft": "#414243",
            "--ink": "#e6e6e6", "--muted": "#676b6d", "--line": "#343536", "--accent": "#19f9d8",
            "--accent-soft": "rgba(25, 249, 216, 0.15)", "--warning": "#ff75b5"
        } "##),
        ("snazzy", "Snazzy", r##" {
            "--bg": "#282a36", "--bg-accent": "#1e2029", "--surface": "#343746", "--surface-soft": "#44475a",
            "--ink": "#eff0eb", "--muted": "#6272a4", "--line": "#44475a", "--accent": "#5af78e",
            "--accent-soft": "rgba(90, 247, 142, 0.15)", "--warning": "#ff5c57"
        } "##),
        ("aura-dark", "Aura Dark", r##" {
            "--bg": "#15141b", "--bg-accent": "#111016", "--surface": "#1d1b24", "--surface-soft": "#292633",
            "--ink": "#edecee", "--muted": "#6d6d6d", "--line": "#292633", "--accent": "#61ffca",
            "--accent-soft": "rgba(97, 255, 202, 0.15)", "--warning": "#ff6767"
        } "##),
        ("miramare", "Miramare", r##" {
            "--bg": "#242424", "--bg-accent": "#1d1d1d", "--surface": "#323232", "--surface-soft": "#444444",
            "--ink": "#e6d6ac", "--muted": "#8b8b8b", "--line": "#444444", "--accent": "#a7c080",
            "--accent-soft": "rgba(167, 192, 128, 0.15)", "--warning": "#e67e80"
        } "##),
        ("sonokai-espresso", "Sonokai Espresso", r##" {
            "--bg": "#2d2a2e", "--bg-accent": "#221f22", "--surface": "#403e41", "--surface-soft": "#5b595c",
            "--ink": "#e3e1e4", "--muted": "#727072", "--line": "#403e41", "--accent": "#a9dc76",
            "--accent-soft": "rgba(169, 220, 118, 0.15)", "--warning": "#ff6188"
        } "##),
        ("edge-dark", "Edge Dark", r##" {
            "--bg": "#2c2e34", "--bg-accent": "#202227", "--surface": "#33363d", "--surface-soft": "#3e4249",
            "--ink": "#c5cdd9", "--muted": "#7f8490", "--line": "#3e4249", "--accent": "#7cc844",
            "--accent-soft": "rgba(124, 200, 68, 0.15)", "--warning": "#ec7279"
        } "##),
        ("boxy-ocean", "Boxy Ocean", r##" {
            "--bg": "#1a2b34", "--bg-accent": "#111c24", "--surface": "#233d45", "--surface-soft": "#334e56",
            "--ink": "#d8dee9", "--muted": "#65737e", "--line": "#334e56", "--accent": "#00bcd4",
            "--accent-soft": "rgba(0, 188, 212, 0.15)", "--warning": "#ff5252"
        } "##),
        ("zenburn", "Zenburn", r##" {
            "--bg": "#3f3f3f", "--bg-accent": "#2f2f2f", "--surface": "#4f4f4f", "--surface-soft": "#5f5f5f",
            "--ink": "#dcdccc", "--muted": "#7f9f7f", "--line": "#4f4f4f", "--accent": "#f0dfaf",
            "--accent-soft": "rgba(240, 223, 175, 0.15)", "--warning": "#cc9393"
        } "##),
        ("tomorrow-night-bright", "Tomorrow Night Bright", r##" {
            "--bg": "#000000", "--bg-accent": "#080808", "--surface": "#1a1a1a", "--surface-soft": "#2a2a2a",
            "--ink": "#eaeaea", "--muted": "#969896", "--line": "#2a2a2a", "--accent": "#7aa6da",
            "--accent-soft": "rgba(122, 166, 218, 0.15)", "--warning": "#d54e53"
        } "##),
        ("blackboard", "Blackboard", r##" {
            "--bg": "#0c1021", "--bg-accent": "#080b18", "--surface": "#1b2130", "--surface-soft": "#252c40",
            "--ink": "#f8f8f8", "--muted": "#888888", "--line": "#252c40", "--accent": "#fbde2d",
            "--accent-soft": "rgba(251, 222, 45, 0.15)", "--warning": "#ff628c"
        } "##),
        ("material-palenight", "Material Palenight", r##" {
            "--bg": "#292d3e", "--bg-accent": "#1b1e2b", "--surface": "#32374d", "--surface-soft": "#444b63",
            "--ink": "#a6accd", "--muted": "#676e95", "--line": "#444b63", "--accent": "#c792ea",
            "--accent-soft": "rgba(199, 146, 234, 0.15)", "--warning": "#ff5370"
        } "##),
        ("kimbie-dark", "Kimbie Dark", r##" {
            "--bg": "#221a0f", "--bg-accent": "#1a130c", "--surface": "#362719", "--surface-soft": "#4d3624",
            "--ink": "#d3af86", "--muted": "#84613d", "--line": "#4d3624", "--accent": "#f06431",
            "--accent-soft": "rgba(240, 100, 49, 0.15)", "--warning": "#dc3958"
        } "##),
        ("minimum-dark", "Minimum Dark", r##" {
            "--bg": "#000000", "--bg-accent": "#0a0a0a", "--surface": "#111111", "--surface-soft": "#1a1a1a",
            "--ink": "#ffffff", "--muted": "#666666", "--line": "#222222", "--accent": "#ffffff",
            "--accent-soft": "rgba(255, 255, 255, 0.1)", "--warning": "#ff0000"
        } "##),
        ("solarized-dark", "Solarized Dark", r##" {
            "--bg": "#002b36", "--bg-accent": "#073642", "--surface": "#002b36", "--surface-soft": "#073642",
            "--ink": "#839496", "--muted": "#586e75", "--line": "#073642", "--accent": "#268bd2",
            "--accent-soft": "rgba(38, 139, 210, 0.15)", "--warning": "#dc322f"
        } "##),
        ("tomorrow-night", "Tomorrow Night", r##" {
            "--bg": "#1d1f21", "--bg-accent": "#151718", "--surface": "#282a2e", "--surface-soft": "#373b41",
            "--ink": "#c5c8c6", "--muted": "#969896", "--line": "#373b41", "--accent": "#81a2be",
            "--accent-soft": "rgba(129, 162, 190, 0.15)", "--warning": "#cc6666"
        } "##),

        // --- 3. ELITE LIGHT COLLECTION ---
        ("everforest-light", "Everforest Light", r##" {
            "--bg": "#fdf6e3", "--bg-accent": "#fefcf0", "--surface": "#f8f0dc", "--surface-soft": "#efebd4",
            "--ink": "#5c6a72", "--muted": "#939f91", "--line": "#e8e5d5", "--accent": "#8da101",
            "--accent-soft": "rgba(141, 161, 1, 0.1)", "--warning": "#f85552"
        } "##),
        ("catppuccin-latte", "Catppuccin Latte", r##" {
            "--bg": "#eff1f5", "--bg-accent": "#e6e9ef", "--surface": "#ccd0da", "--surface-soft": "#bcc0cc",
            "--ink": "#4c4f69", "--muted": "#7c7f93", "--line": "#bcc0cc", "--accent": "#1e66f5",
            "--accent-soft": "rgba(30, 102, 245, 0.1)", "--warning": "#d20f39"
        } "##),
        ("nord-light", "Nord Light", r##" {
            "--bg": "#eceff4", "--bg-accent": "#e5e9f0", "--surface": "#d8dee9", "--surface-soft": "#cdd3de",
            "--ink": "#2e3440", "--muted": "#4c566a", "--line": "#d8dee9", "--accent": "#5e81ac",
            "--accent-soft": "rgba(94, 129, 172, 0.1)", "--warning": "#bf616a"
        } "##),
        ("one-light", "One Light", r##" {
            "--bg": "#fafafa", "--bg-accent": "#f0f0f0", "--surface": "#ffffff", "--surface-soft": "#e5e5e6",
            "--ink": "#383a42", "--muted": "#a0a1a7", "--line": "#dbdbdc", "--accent": "#4078f2",
            "--accent-soft": "rgba(64, 120, 242, 0.1)", "--warning": "#e45649"
        } "##),
        ("solarized-light", "Solarized Light", r##" {
            "--bg": "#fdf6e3", "--bg-accent": "#eee8d5", "--surface": "#fdf6e3", "--surface-soft": "#eee8d5",
            "--ink": "#657b83", "--muted": "#93a1a1", "--line": "#d5c4a1", "--accent": "#268bd2",
            "--accent-soft": "rgba(38, 139, 210, 0.1)", "--warning": "#dc322f"
        } "##),
        ("paper-color-light", "PaperColor Light", r##" {
            "--bg": "#f5f5f5", "--bg-accent": "#eeeeee", "--surface": "#ffffff", "--surface-soft": "#e4e4e4",
            "--ink": "#444444", "--muted": "#878787", "--line": "#d0d0d0", "--accent": "#005f87",
            "--accent-soft": "rgba(0, 95, 135, 0.1)", "--warning": "#df0000"
        } "##),
        ("rose-pine-dawn", "Rosé Pine Dawn", r##" {
            "--bg": "#faf4ed", "--bg-accent": "#fffaf3", "--surface": "#f2e9e1", "--surface-soft": "#ebe1d7",
            "--ink": "#575279", "--muted": "#797593", "--line": "#dfdad9", "--accent": "#d7827e",
            "--accent-soft": "rgba(215, 130, 126, 0.1)", "--warning": "#b4637a"
        } "##),
        ("gruvbox-light", "Gruvbox Light", r##" {
            "--bg": "#fbf1c7", "--bg-accent": "#f2e5bc", "--surface": "#ebdbb2", "--surface-soft": "#d5c4a1",
            "--ink": "#3c3836", "--muted": "#928374", "--line": "#d5c4a1", "--accent": "#af3a03",
            "--accent-soft": "rgba(175, 58, 3, 0.1)", "--warning": "#9d0006"
        } "##),
        ("ayu-light", "Ayu Light", r##" {
            "--bg": "#fafafa", "--bg-accent": "#f3f3f3", "--surface": "#ffffff", "--surface-soft": "#f0f0f0",
            "--ink": "#5c6773", "--muted": "#abb0b6", "--line": "#e0e0e0", "--accent": "#f29718",
            "--accent-soft": "rgba(242, 151, 24, 0.1)", "--warning": "#f07178"
        } "##),
        ("material-lighter", "Material Lighter", r##" {
            "--bg": "#fafafa", "--bg-accent": "#f5f5f5", "--surface": "#ffffff", "--surface-soft": "#f0f0f0",
            "--ink": "#90a4ae", "--muted": "#b0bec5", "--line": "#e1e2e1", "--accent": "#80cbc4",
            "--accent-soft": "rgba(128, 203, 196, 0.1)", "--warning": "#ff5370"
        } "##),
        ("iceberg-light", "Iceberg Light", r##" {
            "--bg": "#e8e9ec", "--bg-accent": "#dcdfe4", "--surface": "#ffffff", "--surface-soft": "#d2d5db",
            "--ink": "#33374c", "--muted": "#8389a3", "--line": "#cad0de", "--accent": "#2d539e",
            "--accent-soft": "rgba(45, 83, 158, 0.1)", "--warning": "#cc5151"
        } "##),
        ("tokyo-night-day", "Tokyo Night Day", r##" {
            "--bg": "#e1e2e7", "--bg-accent": "#d5d6db", "--surface": "#cfd0d7", "--surface-soft": "#b9becf",
            "--ink": "#3760bf", "--muted": "#686f9a", "--line": "#adb5bd", "--accent": "#2e7de9",
            "--accent-soft": "rgba(46, 125, 233, 0.1)", "--warning": "#f52a65"
        } "##),
        ("flat-ui-light", "Flat UI Light", r##" {
            "--bg": "#ecf0f1", "--bg-accent": "#dbe4e6", "--surface": "#ffffff", "--surface-soft": "#dae0e2",
            "--ink": "#2c3e50", "--muted": "#7f8c8d", "--line": "#bdc3c7", "--accent": "#3498db",
            "--accent-soft": "rgba(52, 152, 219, 0.1)", "--warning": "#e74c3c"
        } "##),
        ("minimum-light", "Minimum Light", r##" {
            "--bg": "#ffffff", "--bg-accent": "#fafafa", "--surface": "#ffffff", "--surface-soft": "#f5f5f5",
            "--ink": "#111111", "--muted": "#999999", "--line": "#eeeeee", "--accent": "#000000",
            "--accent-soft": "rgba(0, 0, 0, 0.05)", "--warning": "#ff0000"
        } "##),
        ("noctis-lux", "Noctis Lux", r##" {
            "--bg": "#fbfbfb", "--bg-accent": "#f2f2f2", "--surface": "#ffffff", "--surface-soft": "#e9e9e9",
            "--ink": "#444444", "--muted": "#888888", "--line": "#dddddd", "--accent": "#005f87",
            "--accent-soft": "rgba(0, 95, 135, 0.1)", "--warning": "#df0000"
        } "##),
        ("edge-light", "Edge Light", r##" {
            "--bg": "#fafafa", "--bg-accent": "#f0f0f0", "--surface": "#ffffff", "--surface-soft": "#e8e8e8",
            "--ink": "#5c6a72", "--muted": "#939f91", "--line": "#e0e0e0", "--accent": "#4d6a80",
            "--accent-soft": "rgba(77, 106, 128, 0.1)", "--warning": "#d9534f"
        } "##),

        // --- 4. THE SPECIALTY & DESIGNER SERIES ---
        ("atom-dark", "Atom Dark", r##" {
            "--bg": "#1d2125", "--bg-accent": "#181a1f", "--surface": "#21252b", "--surface-soft": "#282c34",
            "--ink": "#abb2bf", "--muted": "#5c6370", "--line": "#3e4451", "--accent": "#61afef",
            "--accent-soft": "rgba(97, 175, 239, 0.15)", "--warning": "#e06c75"
        } "##),
        ("cobalt-classic", "Cobalt Classic", r##" {
            "--bg": "#002240", "--bg-accent": "#001a33", "--surface": "#003b70", "--surface-soft": "#004b8d",
            "--ink": "#ffffff", "--muted": "#888888", "--line": "#004b8d", "--accent": "#ff9d00",
            "--accent-soft": "rgba(255, 157, 0, 0.15)", "--warning": "#ff628c"
        } "##),
        ("monokai-octagon", "Monokai Octagon", r##" {
            "--bg": "#282a3a", "--bg-accent": "#1e1f2b", "--surface": "#3a3d4d", "--surface-soft": "#4d5166",
            "--ink": "#eaf2f1", "--muted": "#696d77", "--line": "#3a3d4d", "--accent": "#ffcc66",
            "--accent-soft": "rgba(255, 204, 102, 0.15)", "--warning": "#f92672"
        } "##),
        ("dracula-soft", "Dracula Soft", r##" {
            "--bg": "#282a36", "--bg-accent": "#21222c", "--surface": "#343746", "--surface-soft": "#44475a",
            "--ink": "#f8f8f2", "--muted": "#6272a4", "--line": "#44475a", "--accent": "#bd93f9",
            "--accent-soft": "rgba(189, 147, 249, 0.12)", "--warning": "#ff5555"
        } "##),
        ("night-owl-light", "Night Owl Light", r##" {
            "--bg": "#fbfbfb", "--bg-accent": "#f0f0f0", "--surface": "#ffffff", "--surface-soft": "#e8e8e8",
            "--ink": "#403f53", "--muted": "#7a8181", "--line": "#d3d3d3", "--accent": "#2064df",
            "--accent-soft": "rgba(32, 100, 223, 0.1)", "--warning": "#d3423e"
        } "##),
        ("synthwave-no-glow", "SynthWave '84 (Flat)", r##" {
            "--bg": "#262335", "--bg-accent": "#232136", "--surface": "#2d2b45", "--surface-soft": "#3d3b5c",
            "--ink": "#ffffff", "--muted": "#848bbd", "--line": "#3d3b5c", "--accent": "#36f9f6",
            "--accent-soft": "rgba(54, 249, 246, 0.15)", "--warning": "#fe4450"
        } "##),
        ("cyberpunk-day", "Cyberpunk Day", r##" {
            "--bg": "#f5f5f5", "--bg-accent": "#eeeeee", "--surface": "#ffffff", "--surface-soft": "#e4e4e4",
            "--ink": "#000000", "--muted": "#666666", "--line": "#cccccc", "--accent": "#f300ff",
            "--accent-soft": "rgba(243, 0, 255, 0.1)", "--warning": "#ff003c"
        } "##),
        ("kanagawa-lotus", "Kanagawa Lotus", r##" {
            "--bg": "#f2ecbc", "--bg-accent": "#e9e19d", "--surface": "#dcd5ac", "--surface-soft": "#cdc6a1",
            "--ink": "#545464", "--muted": "#717c7c", "--line": "#cdc6a1", "--accent": "#4d699b",
            "--accent-soft": "rgba(77, 105, 155, 0.1)", "--warning": "#c84053"
        } "##),
        ("catppuccin-frappe", "Catppuccin Frappé", r##" {
            "--bg": "#303446", "--bg-accent": "#292c3c", "--surface": "#414559", "--surface-soft": "#51576d",
            "--ink": "#c6d0f5", "--muted": "#838ba7", "--line": "#51576d", "--accent": "#8ca0f3",
            "--accent-soft": "rgba(140, 160, 243, 0.15)", "--warning": "#e78284"
        } "##),
        ("tomorrow-night-blue", "Tomorrow Night Blue", r##" {
            "--bg": "#002451", "--bg-accent": "#001c3d", "--surface": "#00346e", "--surface-soft": "#003f8e",
            "--ink": "#ffffff", "--muted": "#7285b7", "--line": "#003f8e", "--accent": "#ffc58a",
            "--accent-soft": "rgba(255, 197, 138, 0.15)", "--warning": "#ff9da4"
        } "##),
        ("boxy-tomorrow", "Boxy Tomorrow", r##" {
            "--bg": "#1d1f21", "--bg-accent": "#151718", "--surface": "#282a2e", "--surface-soft": "#373b41",
            "--ink": "#c5c8c6", "--muted": "#969896", "--line": "#373b41", "--accent": "#b294bb",
            "--accent-soft": "rgba(178, 148, 187, 0.15)", "--warning": "#cc6666"
        } "##),
        ("parity-light", "Parity Light", r##" {
            "--bg": "#f0f0f0", "--bg-accent": "#e0e0e0", "--surface": "#ffffff", "--surface-soft": "#d0d0d0",
            "--ink": "#333333", "--muted": "#888888", "--line": "#cccccc", "--accent": "#009900",
            "--accent-soft": "rgba(0, 153, 0, 0.1)", "--warning": "#cc0000"
        } "##),
        ("laserwave-hard", "LaserWave Hard", r##" {
            "--bg": "#1a1621", "--bg-accent": "#110f18", "--surface": "#241f2d", "--surface-soft": "#322b3e",
            "--ink": "#ffffff", "--muted": "#918175", "--line": "#322b3e", "--accent": "#eb64b9",
            "--accent-soft": "rgba(235, 100, 185, 0.15)", "--warning": "#74dfc4"
        } "##),
        ("sonokai-shusia", "Sonokai Shusia", r##" {
            "--bg": "#2d2a2e", "--bg-accent": "#221f22", "--surface": "#373539", "--surface-soft": "#49464e",
            "--ink": "#e3e1e4", "--muted": "#727072", "--line": "#373539", "--accent": "#fc9867",
            "--accent-soft": "rgba(252, 152, 103, 0.15)", "--warning": "#ff6188"
        } "##),
        ("edge-soft-dark", "Edge Soft Dark", r##" {
            "--bg": "#262729", "--bg-accent": "#1d1e1f", "--surface": "#2c2e31", "--surface-soft": "#3a3d42",
            "--ink": "#c5cdd9", "--muted": "#7f8490", "--line": "#2c2e31", "--accent": "#3dbbb0",
            "--accent-soft": "rgba(61, 187, 176, 0.15)", "--warning": "#ec7279"
        } "##),
    ];

    // Clear old built-in themes
    conn.execute("DELETE FROM themes WHERE is_builtin = 1", [])?;

    for (id, name, config) in builtin_themes {
        conn.execute(
            "INSERT INTO themes (id, name, config, is_builtin) VALUES (?1, ?2, ?3, 1)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, config=excluded.config
             ON CONFLICT(name) DO UPDATE SET id=excluded.id, config=excluded.config",
            [id, name, config],
        )?;
    }

    // --- MIGRATIONS (Continued) ---
    let table_sql: String = conn.query_row("SELECT sql FROM sqlite_master WHERE type='table' AND name='jobs'", [], |row| row.get(0)).unwrap_or_default();
    if table_sql.contains("CHECK(employment_type IN") || table_sql.contains("CHECK(work_model IN") {
        println!("Migrating 'jobs' table to flexible schema...");
        conn.execute("PRAGMA foreign_keys=OFF", [])?;
        let _ = (|| -> Result<()> {
            conn.execute("BEGIN TRANSACTION", [])?;
            conn.execute("DROP TRIGGER IF EXISTS update_jobs_modtime", [])?;
            conn.execute("DROP TABLE IF EXISTS jobs_old", [])?;
            conn.execute("ALTER TABLE jobs RENAME TO jobs_old", [])?;
            conn.execute(
                "CREATE TABLE jobs (
                    id TEXT PRIMARY KEY, company_name TEXT NOT NULL, job_title TEXT NOT NULL,
                    work_model TEXT DEFAULT 'Remote', employment_type TEXT DEFAULT 'Full-time',
                    status TEXT DEFAULT 'Drafting', raw_jd TEXT NOT NULL, requirements TEXT,
                    core_responsibilities TEXT, custom_instruction TEXT, reference_name TEXT,
                    reference_email TEXT, social_link TEXT, created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
                )",
                [],
            )?;
            conn.execute("INSERT INTO jobs SELECT id, company_name, job_title, work_model, employment_type, status, raw_jd, requirements, core_responsibilities, custom_instruction, reference_name, reference_email, social_link, created_at, updated_at FROM jobs_old", [])?;
            conn.execute("DROP TABLE jobs_old", [])?;
            conn.execute("CREATE TRIGGER update_jobs_modtime AFTER UPDATE ON jobs BEGIN UPDATE jobs SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id; END;", [])?;
            conn.execute("COMMIT", [])?;
            Ok(())
        })();
        conn.execute("PRAGMA foreign_keys=ON", [])?;
    }

    let cols = ["reference_name", "reference_email", "social_link", "custom_instruction", "requirements", "core_responsibilities", "job_url", "salary", "applied_date", "interview_date", "offer_date", "rejected_date", "joining_date", "base_resume_id", "base_cl_id"];
    for col in cols {
        if !conn.prepare("PRAGMA table_info(jobs)")?.query_map([], |row| row.get(1))?.collect::<Result<Vec<String>, _>>()?.contains(&col.to_string()) {
            let _ = conn.execute(&format!("ALTER TABLE jobs ADD COLUMN {} TEXT", col), []);
        }
    }

    Ok(conn)
}
