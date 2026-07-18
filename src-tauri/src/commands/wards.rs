use super::*;
use crate::db::read_banned_words;
use crate::helpers::timestamp;
use crate::llm::scan_wards;

#[tauri::command]
pub fn wards_list(project_path: String) -> CommandResult<Vec<BannedWord>> {
    let connection = open_project_database(&project_path)?;
    read_banned_words(&connection)
}

#[tauri::command]
pub fn wards_add(request: WardPhraseRequest) -> CommandResult<Vec<BannedWord>> {
    let connection = open_project_database(&request.project_path)?;
    let value = request.value.trim();
    if value.is_empty() {
        return Err("Ward phrase cannot be empty.".to_string());
    }

    let severity = match request.severity.as_deref().unwrap_or("warn") {
        "block" => "block",
        _ => "warn",
    };
    let now = timestamp();
    connection
        .execute(
            r#"
            INSERT INTO banned_words (id, value, severity, is_default, created_at, updated_at)
            VALUES (?1, ?2, ?3, 0, ?4, ?4)
            ON CONFLICT(value) DO UPDATE SET severity = excluded.severity, updated_at = excluded.updated_at
            "#,
            params![format!("ward_{}", crate::helpers::timestamp_nanos()), value, severity, now],
        )
        .map_err(|error| format!("Could not save ward phrase: {error}"))?;

    read_banned_words(&connection)
}

#[tauri::command]
pub fn wards_remove(project_path: String, id: String) -> CommandResult<Vec<BannedWord>> {
    let connection = open_project_database(&project_path)?;
    connection
        .execute("DELETE FROM banned_words WHERE id = ?1", params![id])
        .map_err(|error| format!("Could not remove ward phrase: {error}"))?;
    read_banned_words(&connection)
}

#[tauri::command]
pub fn wards_scan(request: WardScanRequest) -> CommandResult<WardScanResponse> {
    let connection = open_project_database(&request.project_path)?;
    let words = read_banned_words(&connection)?;
    Ok(scan_wards(&words, &request.text))
}
