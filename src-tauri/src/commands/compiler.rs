use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn save_compiler_state(
    state: State<'_, AppState>,
    latex_content: String,
) -> Result<(), String> {
    let mut db_guard = state.db.lock().map_err(|e| format!("Mutex error: {}", e))?;
    let conn = db_guard.as_mut().ok_or("Database connection lost")?;

    conn.execute(
        "INSERT INTO compiler_state (id, latex_content) 
         VALUES (1, ?1) 
         ON CONFLICT(id) DO UPDATE SET 
            latex_content = excluded.latex_content,
            updated_at = CURRENT_TIMESTAMP",
        [&latex_content],
    )
    .map_err(|e| format!("Database error: {}", e))?;

    state.mark_dirty();
    Ok(())
}

#[tauri::command]
pub async fn get_compiler_state(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let mut db_guard = state.db.lock().map_err(|e| format!("Mutex error: {}", e))?;
    let conn = db_guard.as_mut().ok_or("Database connection lost")?;

    let mut stmt = conn
        .prepare("SELECT latex_content FROM compiler_state WHERE id = 1")
        .map_err(|e| e.to_string())?;

    let content: Option<String> = match stmt.query_row([], |row| row.get(0)) {
        Ok(v) => Some(v),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(e.to_string()),
    };

    Ok(content)
}
