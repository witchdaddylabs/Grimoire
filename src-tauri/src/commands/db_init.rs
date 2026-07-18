use super::*;
use crate::db::read_vault_tree;

#[tauri::command]
pub fn db_init(project_path: String) -> CommandResult<ProjectMetadata> {
    let project_dir = super::validate_project_dir(PathBuf::from(project_path))?;
    let metadata = super::read_metadata(&project_dir)?;
    super::initialise_database(&metadata, false)?;
    Ok(metadata)
}

#[tauri::command]
pub fn db_get_vault_tree(project_path: String) -> CommandResult<VaultTreeResponse> {
    let project_dir = super::validate_project_dir(PathBuf::from(project_path))?;
    let metadata = super::read_metadata(&project_dir)?;
    super::initialise_database(&metadata, false)?;

    let connection = Connection::open(&metadata.database_path)
        .map_err(|error| format!("Could not open SQLite database: {error}"))?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| format!("Could not enable SQLite foreign keys: {error}"))?;

    read_vault_tree(&connection)
}
