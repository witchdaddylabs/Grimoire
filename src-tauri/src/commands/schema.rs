use crate::helpers::{timestamp, timestamp_nanos};
use rusqlite::{params, Connection};

use crate::errors::CommandResult;

pub const SCHEMA_VERSION: i64 = 2;

pub const INITIAL_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS project_metadata (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS wings (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS halls (
  id TEXT PRIMARY KEY,
  wing_id TEXT NOT NULL,
  name TEXT NOT NULL,
  description TEXT,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (wing_id) REFERENCES wings(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_halls_wing_id ON halls(wing_id);

CREATE TABLE IF NOT EXISTS rooms (
  id TEXT PRIMARY KEY,
  hall_id TEXT NOT NULL,
  name TEXT NOT NULL,
  description TEXT,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (hall_id) REFERENCES halls(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_rooms_hall_id ON rooms(hall_id);

CREATE TABLE IF NOT EXISTS drawers (
  id TEXT PRIMARY KEY,
  room_id TEXT NOT NULL,
  name TEXT NOT NULL,
  description TEXT,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (room_id) REFERENCES rooms(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_drawers_room_id ON drawers(room_id);

CREATE TABLE IF NOT EXISTS items (
  id TEXT PRIMARY KEY,
  drawer_id TEXT NOT NULL,
  title TEXT NOT NULL,
  item_type TEXT NOT NULL CHECK (
    item_type IN (
      'chapter',
      'scene',
      'character',
      'location',
      'lore',
      'timeline',
      'faction',
      'research',
      'note'
    )
  ),
  content TEXT,
  plain_text TEXT,
  word_count INTEGER NOT NULL DEFAULT 0,
  memory_enabled INTEGER NOT NULL DEFAULT 1,
  source_kind TEXT NOT NULL DEFAULT 'manual',
  source_path TEXT,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  archived_at TEXT,
  FOREIGN KEY (drawer_id) REFERENCES drawers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_items_drawer_id ON items(drawer_id);
CREATE INDEX IF NOT EXISTS idx_items_item_type ON items(item_type);
CREATE INDEX IF NOT EXISTS idx_items_memory_enabled ON items(memory_enabled);
CREATE INDEX IF NOT EXISTS idx_items_updated_at ON items(updated_at);
CREATE INDEX IF NOT EXISTS idx_items_archived_at ON items(archived_at);

CREATE TABLE IF NOT EXISTS item_chunks (
  id TEXT PRIMARY KEY,
  item_id TEXT NOT NULL,
  chunk_index INTEGER NOT NULL,
  text TEXT NOT NULL,
  word_count INTEGER NOT NULL DEFAULT 0,
  start_offset INTEGER,
  end_offset INTEGER,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (item_id) REFERENCES items(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_item_chunks_item_id ON item_chunks(item_id);
CREATE INDEX IF NOT EXISTS idx_item_chunks_chunk_index ON item_chunks(item_id, chunk_index);

CREATE VIRTUAL TABLE IF NOT EXISTS item_chunks_fts
USING fts5(
  chunk_id,
  item_id,
  title,
  item_type,
  vault_path,
  text
);

CREATE TABLE IF NOT EXISTS banned_words (
  id TEXT PRIMARY KEY,
  value TEXT NOT NULL UNIQUE,
  severity TEXT NOT NULL DEFAULT 'warn' CHECK (
    severity IN ('warn', 'block')
  ),
  is_default INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_banned_words_value ON banned_words(value);

CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
"#;

pub fn seed_vault_demo_data(connection: &Connection) -> CommandResult<()> {
    let now = timestamp();

    // Check if data already exists
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM wings", [], |row| row.get(0))
        .unwrap_or(0);
    if count > 0 {
        return Ok(());
    }

    let wing_id = format!("wing_{}", timestamp_nanos());
    let hall_id = format!("hall_{}", timestamp_nanos());
    let room_id = format!("room_{}", timestamp_nanos());
    let drawer_id = format!("drawer_{}", timestamp_nanos());
    let item_id = format!("item_{}", timestamp_nanos());

    connection.execute_batch(&format!(
        r#"
        INSERT INTO wings (id, name, description, sort_order, created_at, updated_at)
        VALUES ('{wing_id}', 'The Grimoire', 'Your writing vault', 0, '{now}', '{now}');

        INSERT INTO halls (id, wing_id, name, description, sort_order, created_at, updated_at)
        VALUES ('{hall_id}', '{wing_id}', 'Manuscript', 'Your manuscript chapters', 0, '{now}', '{now}');

        INSERT INTO rooms (id, hall_id, name, description, sort_order, created_at, updated_at)
        VALUES ('{room_id}', '{hall_id}', 'Chapters', 'Draft chapters and scenes', 0, '{now}', '{now}');

        INSERT INTO drawers (id, room_id, name, description, sort_order, created_at, updated_at)
        VALUES ('{drawer_id}', '{room_id}', 'Drafts', 'Working drafts', 0, '{now}', '{now}');

        INSERT INTO items (id, drawer_id, title, item_type, content, plain_text, word_count, source_kind, sort_order, created_at, updated_at)
        VALUES ('{item_id}', '{drawer_id}', 'Chapter 1', 'chapter', 'Begin your story here...', 'Begin your story here...', 5, 'manual', 0, '{now}', '{now}');
        "#,
    ))
    .map_err(|error| format!("Could not seed demo data: {error}"))?;

    Ok(())
}

pub fn run_migrations(connection: &mut Connection) -> CommandResult<()> {
    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
              version INTEGER PRIMARY KEY,
              name TEXT NOT NULL,
              applied_at TEXT NOT NULL
            );
            "#,
        )
        .map_err(|error| format!("Could not prepare migration table: {error}"))?;

    let already_applied: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
            params![SCHEMA_VERSION],
            |row| row.get(0),
        )
        .map_err(|error| format!("Could not read migration state: {error}"))?;

    connection
        .execute_batch(INITIAL_SCHEMA)
        .map_err(|error| format!("Could not apply initial schema: {error}"))?;
    ensure_items_archive_column(connection)?;

    if already_applied == 0 {
        connection
            .execute(
                "INSERT OR IGNORE INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
                params![SCHEMA_VERSION, "archive_items", timestamp()],
            )
            .map_err(|error| format!("Could not record schema migration: {error}"))?;
    }

    Ok(())
}

fn ensure_items_archive_column(connection: &Connection) -> CommandResult<()> {
    let mut statement = connection
        .prepare("PRAGMA table_info(items)")
        .map_err(|error| format!("Could not inspect items table: {error}"))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| format!("Could not inspect items table columns: {error}"))?;

    let mut has_archived_at = false;
    for column in columns {
        if column.map_err(|error| format!("Could not read items table column: {error}"))?
            == "archived_at"
        {
            has_archived_at = true;
            break;
        }
    }

    if !has_archived_at {
        connection
            .execute_batch("ALTER TABLE items ADD COLUMN archived_at TEXT;")
            .map_err(|error| format!("Could not add item archive column: {error}"))?;
    }

    connection
        .execute_batch("CREATE INDEX IF NOT EXISTS idx_items_archived_at ON items(archived_at);")
        .map_err(|error| format!("Could not prepare item archive index: {error}"))?;

    Ok(())
}

pub fn upsert_project_metadata(
    connection: &Connection,
    metadata: &crate::models::ProjectMetadata,
) -> CommandResult<()> {
    let updated_at = timestamp();
    let values = [
        ("name", metadata.name.clone()),
        ("app_version", metadata.app_version.clone()),
        ("schema_version", metadata.schema_version.to_string()),
        ("project_path", metadata.project_path.clone()),
        ("database_path", metadata.database_path.clone()),
    ];

    for (key, value) in values {
        connection
            .execute(
                r#"
                INSERT INTO project_metadata (key, value, updated_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(key) DO UPDATE SET
                  value = excluded.value,
                  updated_at = excluded.updated_at
                "#,
                params![key, value, updated_at],
            )
            .map_err(|error| format!("Could not write project metadata to SQLite: {error}"))?;
    }

    Ok(())
}
