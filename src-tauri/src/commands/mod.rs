pub mod ai;
pub mod db_init;
pub mod export;
pub mod ollama;
pub mod project;
pub mod schema;
pub mod search;
pub mod storyplan;
pub mod vault;
pub mod wards;

use crate::errors::CommandResult;
use crate::helpers::timestamp;
use crate::models::*;
use rusqlite::{params, Connection};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const DATABASE_FILE: &str = "grimoire.sqlite";
const METADATA_FILE: &str = "metadata.json";

// ── Project / Database helpers ──

pub fn default_projects_dir() -> CommandResult<PathBuf> {
    let home = env::var_os("HOME").ok_or("Could not resolve HOME for project storage")?;
    Ok(PathBuf::from(home)
        .join("Documents")
        .join("Grimoire Projects"))
}

pub fn project_folder_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric()
                || *character == ' '
                || *character == '-'
                || *character == '_'
        })
        .collect();

    let trimmed = cleaned.trim();
    let base = if trimmed.is_empty() {
        "Untitled Grimoire"
    } else {
        trimmed
    };

    format!("{base}.grimoire")
}

pub fn validate_project_dir(project_dir: PathBuf) -> CommandResult<PathBuf> {
    if project_dir.extension().and_then(|value| value.to_str()) != Some("grimoire") {
        return Err("Expected a .grimoire project folder".to_string());
    }

    if !project_dir.is_dir() {
        return Err(format!(
            "Project folder does not exist: {}",
            project_dir.display()
        ));
    }

    Ok(project_dir)
}

pub fn load_or_create_metadata(project_dir: &Path, name: &str) -> CommandResult<ProjectMetadata> {
    let metadata_path = project_dir.join(METADATA_FILE);
    if metadata_path.exists() {
        return read_metadata(project_dir);
    }

    let now = timestamp();
    let metadata = ProjectMetadata {
        name: name.trim().to_string(),
        app_version: APP_VERSION.to_string(),
        schema_version: self::schema::SCHEMA_VERSION,
        project_path: project_dir.to_string_lossy().to_string(),
        database_path: project_dir
            .join(DATABASE_FILE)
            .to_string_lossy()
            .to_string(),
        created_at: now.clone(),
        updated_at: now,
    };

    write_metadata(&metadata)?;
    Ok(metadata)
}

pub fn read_metadata(project_dir: &Path) -> CommandResult<ProjectMetadata> {
    let metadata_path = project_dir.join(METADATA_FILE);
    let raw = fs::read_to_string(&metadata_path).map_err(|error| {
        format!(
            "Could not read project metadata at {}: {error}",
            metadata_path.display()
        )
    })?;
    serde_json::from_str(&raw).map_err(|error| format!("Could not parse project metadata: {error}"))
}

pub fn write_metadata(metadata: &ProjectMetadata) -> CommandResult<()> {
    let project_dir = PathBuf::from(&metadata.project_path);
    let metadata_path = project_dir.join(METADATA_FILE);
    let raw = serde_json::to_string_pretty(metadata)
        .map_err(|error| format!("Could not serialize project metadata: {error}"))?;
    fs::write(&metadata_path, raw)
        .map_err(|error| format!("Could not write project metadata: {error}"))
}

pub fn initialise_database(
    metadata: &ProjectMetadata,
    seed_demo: bool,
) -> CommandResult<ProjectMetadata> {
    let mut connection = Connection::open(&metadata.database_path)
        .map_err(|error| format!("Could not open SQLite database: {error}"))?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| format!("Could not enable SQLite foreign keys: {error}"))?;

    self::schema::run_migrations(&mut connection)?;

    // Projects written by older builds can carry a stale schema version in
    // metadata.json (Codex catch, 2026-08). Migrations have just run, so bump
    // the metadata to match reality before it is persisted or returned.
    let mut metadata = metadata.clone();
    if metadata.schema_version < self::schema::SCHEMA_VERSION {
        metadata.schema_version = self::schema::SCHEMA_VERSION;
        metadata.updated_at = timestamp();
        write_metadata(&metadata)?;
    }

    self::schema::upsert_project_metadata(&connection, &metadata)?;
    crate::llm::seed_default_banned_words(&connection)?;

    if seed_demo {
        self::schema::seed_vault_demo_data(&connection)?;
    }

    Ok(metadata)
}

pub fn open_project_database(project_path: &str) -> CommandResult<Connection> {
    let project_dir = validate_project_dir(PathBuf::from(project_path))?;
    let metadata = read_metadata(&project_dir)?;
    initialise_database(&metadata, false)?;
    let connection = Connection::open(&metadata.database_path)
        .map_err(|error| format!("Could not open SQLite database: {error}"))?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| format!("Could not enable SQLite foreign keys: {error}"))?;
    Ok(connection)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression test (Codex catch, 2026-08): a v2 project opened after the
    /// v3 migration must end up reporting schema version 3 in metadata.json,
    /// not the stale value it was written with.
    #[test]
    fn initialise_database_bumps_stale_metadata_version() {
        let temp_dir = std::env::temp_dir().join(format!(
            "grimoire_schema_bump_test_{}",
            crate::helpers::timestamp_nanos()
        ));
        fs::create_dir_all(&temp_dir).unwrap();

        let database_path = temp_dir.join("grimoire.sqlite");
        let metadata = ProjectMetadata {
            name: "Legacy Project".to_string(),
            app_version: "0.2.0".to_string(),
            schema_version: 2,
            project_path: temp_dir.to_string_lossy().to_string(),
            database_path: database_path.to_string_lossy().to_string(),
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
        };
        write_metadata(&metadata).unwrap();

        let bumped = initialise_database(&metadata, false).unwrap();
        assert_eq!(bumped.schema_version, self::schema::SCHEMA_VERSION);

        let reloaded = read_metadata(&temp_dir).unwrap();
        assert_eq!(reloaded.schema_version, self::schema::SCHEMA_VERSION);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
