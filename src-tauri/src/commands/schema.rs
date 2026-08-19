use crate::helpers::{timestamp, timestamp_nanos};
use rusqlite::{params, Connection};

use crate::errors::CommandResult;

pub const SCHEMA_VERSION: i64 = 3;

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

/// Schema v3 — Story Plan layer (Fabula-style structure that stays aligned).
/// Applied idempotently after INITIAL_SCHEMA; safe to re-run on v2 projects.
pub const STORY_PLAN_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS story_plans (
  id TEXT PRIMARY KEY,
  project_name TEXT NOT NULL,
  logline TEXT,
  synopsis TEXT,
  status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','outline','drafting','revision','done')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS story_scenes (
  id TEXT PRIMARY KEY,
  plan_id TEXT NOT NULL REFERENCES story_plans(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  setting TEXT,
  summary TEXT,
  linked_item_id TEXT,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS story_beats (
  id TEXT PRIMARY KEY,
  scene_id TEXT NOT NULL REFERENCES story_scenes(id) ON DELETE CASCADE,
  beat_type TEXT NOT NULL CHECK (beat_type IN ('action','dialogue','revelation','conflict','transition','other')),
  content TEXT NOT NULL,
  characters TEXT,
  locked INTEGER NOT NULL DEFAULT 0,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS story_candidates (
  id TEXT PRIMARY KEY,
  target_kind TEXT NOT NULL CHECK (target_kind IN ('plan','scene','beat','script')),
  target_id TEXT NOT NULL,
  provider TEXT NOT NULL,
  model TEXT NOT NULL,
  prompt_summary TEXT,
  candidate_index INTEGER NOT NULL,
  content TEXT NOT NULL,
  /** JSON array of WardScanHit — the hits found when this candidate was scanned. */
  ward_scan_json TEXT NOT NULL DEFAULT '[]',
  /** 1 when wards were actually run for this candidate, 0 when the writer
      opted out. Without this an unscanned candidate is indistinguishable
      from a clean one, and the UI would label it "No slop detected". */
  ward_scanned INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','accepted','rejected')),
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_scenes_plan ON story_scenes(plan_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_beats_scene ON story_beats(scene_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_candidates_target ON story_candidates(target_kind, target_id);
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
                params![2, "archive_items", timestamp()],
            )
            .map_err(|error| format!("Could not record schema migration: {error}"))?;
    }

    // Schema v3 — Story Plan tables. Idempotent (IF NOT EXISTS) so it is safe
    // on brand-new projects and on v2 projects being upgraded alike.
    let story_plan_applied: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
            params![SCHEMA_VERSION],
            |row| row.get(0),
        )
        .map_err(|error| format!("Could not read migration state: {error}"))?;

    connection
        .execute_batch(STORY_PLAN_SCHEMA)
        .map_err(|error| format!("Could not apply story plan schema: {error}"))?;

    // Must run AFTER the CREATE TABLE batch: story_candidates predates the
    // ward_scan_json column, and IF NOT EXISTS cannot add it retroactively.
    ensure_candidate_ward_scan_column(connection)?;

    if story_plan_applied == 0 {
        connection
            .execute(
                "INSERT OR IGNORE INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
                params![SCHEMA_VERSION, "story_plan", timestamp()],
            )
            .map_err(|error| format!("Could not record story plan migration: {error}"))?;
    }

    Ok(())
}

fn ensure_candidate_ward_scan_column(connection: &Connection) -> CommandResult<()> {
    // story_candidates was created in schema v3 WITHOUT ward_scan_json or
    // ward_scanned; both were added later (Sprint 4 / Sprint 5). CREATE TABLE
    // IF NOT EXISTS will not add a column to an existing table, so projects
    // created before those sprints would fail every candidate query with
    // "no such column".
    let table_exists: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='story_candidates'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("Could not inspect story_candidates table: {error}"))?;
    if table_exists == 0 {
        return Ok(());
    }

    let mut statement = connection
        .prepare("PRAGMA table_info(story_candidates)")
        .map_err(|error| format!("Could not inspect story_candidates table: {error}"))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| format!("Could not inspect story_candidates columns: {error}"))?;

    let mut existing: Vec<String> = Vec::new();
    for column in columns {
        existing.push(
            column.map_err(|error| format!("Could not read story_candidates column: {error}"))?,
        );
    }

    if !existing.iter().any(|name| name == "ward_scan_json") {
        connection
            .execute_batch(
                "ALTER TABLE story_candidates ADD COLUMN ward_scan_json TEXT NOT NULL DEFAULT '[]';",
            )
            .map_err(|error| format!("Could not add candidate ward scan column: {error}"))?;
    }

    // Existing rows default to 0 (not scanned), then we backfill any row whose
    // stored scan proves wards DID run. Without the backfill, a migrated
    // candidate carrying blocking hits renders as "Wards not run" and — because
    // the unscanned state is checked before the hits — its Accept button is
    // enabled (Codex P1 on PR #27).
    if !existing.iter().any(|name| name == "ward_scanned") {
        connection
            .execute_batch(
                "ALTER TABLE story_candidates ADD COLUMN ward_scanned INTEGER NOT NULL DEFAULT 0;",
            )
            .map_err(|error| format!("Could not add candidate ward scanned flag: {error}"))?;

        // A non-empty hit array is positive evidence that wards ran. Rows with
        // '[]' stay 0: we genuinely cannot tell whether they were scanned and
        // clean or never scanned, and "unknown" is the honest answer.
        connection
            .execute(
                r#"
                UPDATE story_candidates
                SET ward_scanned = 1
                WHERE ward_scan_json IS NOT NULL
                  AND TRIM(ward_scan_json) NOT IN ('', '[]', 'null')
                "#,
                [],
            )
            .map_err(|error| format!("Could not backfill candidate ward scan state: {error}"))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn
    }

    #[test]
    fn run_migrations_creates_tables() {
        let mut conn = test_db();
        run_migrations(&mut conn).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"wings".to_string()));
        assert!(tables.contains(&"halls".to_string()));
        assert!(tables.contains(&"rooms".to_string()));
        assert!(tables.contains(&"drawers".to_string()));
        assert!(tables.contains(&"items".to_string()));
        assert!(tables.contains(&"item_chunks".to_string()));
        assert!(tables.contains(&"banned_words".to_string()));
        assert!(tables.contains(&"settings".to_string()));
        assert!(tables.contains(&"project_metadata".to_string()));
        assert!(tables.contains(&"schema_migrations".to_string()));
    }

    #[test]
    fn run_migrations_records_version() {
        let mut conn = test_db();
        run_migrations(&mut conn).unwrap();

        let version: i64 = conn
            .query_row(
                "SELECT version FROM schema_migrations WHERE name = 'archive_items'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, 2);

        let story_plan_version: i64 = conn
            .query_row(
                "SELECT version FROM schema_migrations WHERE name = 'story_plan'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(story_plan_version, SCHEMA_VERSION);
    }

    #[test]
    fn run_migrations_idempotent() {
        let mut conn = test_db();
        run_migrations(&mut conn).unwrap();
        run_migrations(&mut conn).unwrap();

        // One row per migration (archive_items + story_plan), never duplicates.
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn ward_scanned_migration_backfills_rows_with_known_hits() {
        // Codex P1 on PR #27: defaulting every migrated row to ward_scanned=0
        // meant a pre-existing candidate carrying BLOCKING hits rendered as
        // "Wards not run" — and because the unscanned branch was checked before
        // the hits, its Accept button was enabled.
        let mut conn = test_db();
        run_migrations(&mut conn).unwrap();

        // Simulate a pre-Sprint-5 project: drop the flag column, leaving rows
        // that carry ward hits but no scanned marker.
        conn.execute_batch(
            "CREATE TABLE candidates_old AS SELECT id, target_kind, target_id, provider, model, prompt_summary, candidate_index, content, ward_scan_json, status, created_at FROM story_candidates;
             DROP TABLE story_candidates;
             ALTER TABLE candidates_old RENAME TO story_candidates;",
        )
        .unwrap();

        let blocking = r#"[{"id":"w1","value":"tapestry","severity":"block","count":1}]"#;
        for (id, scan) in [
            ("c_blocking", blocking),
            (
                "c_warn",
                r#"[{"id":"w2","value":"very","severity":"warn","count":3}]"#,
            ),
            ("c_empty", "[]"),
        ] {
            conn.execute(
                "INSERT INTO story_candidates (id, target_kind, target_id, provider, model, candidate_index, content, ward_scan_json, status, created_at) VALUES (?1, 'scene', 'scene_x', 'ollama', 'llama3.2', 0, 'text', ?2, 'pending', '1')",
                rusqlite::params![id, scan],
            )
            .unwrap();
        }

        // Re-run migrations: the column is re-added and backfilled.
        run_migrations(&mut conn).unwrap();

        let flag = |id: &str| -> i64 {
            conn.query_row(
                "SELECT ward_scanned FROM story_candidates WHERE id = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .unwrap()
        };

        assert_eq!(
            flag("c_blocking"),
            1,
            "stored blocking hits prove wards ran — must not be reported unscanned"
        );
        assert_eq!(flag("c_warn"), 1, "stored warning hits prove wards ran");
        assert_eq!(
            flag("c_empty"),
            0,
            "an empty scan is genuinely ambiguous and stays unscanned"
        );
    }

    #[test]
    fn run_migrations_adds_archive_column() {
        let mut conn = test_db();
        run_migrations(&mut conn).unwrap();

        let mut statement = conn.prepare("PRAGMA table_info(items)").unwrap();
        let columns: Vec<String> = statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(columns.contains(&"archived_at".to_string()));
    }

    #[test]
    fn seed_vault_demo_data_inserts_hierarchy() {
        let conn = test_db();
        conn.execute_batch(INITIAL_SCHEMA).unwrap();
        seed_vault_demo_data(&conn).unwrap();

        let wing_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM wings", [], |row| row.get(0))
            .unwrap();
        assert!(wing_count > 0, "Expected demo wings to be inserted");

        let hall_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM halls", [], |row| row.get(0))
            .unwrap();
        assert!(hall_count > 0, "Expected demo halls to be inserted");

        let item_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))
            .unwrap();
        assert!(item_count > 0, "Expected demo items to be inserted");
    }

    #[test]
    fn seed_vault_demo_data_idempotent() {
        let conn = test_db();
        conn.execute_batch(INITIAL_SCHEMA).unwrap();
        seed_vault_demo_data(&conn).unwrap();
        seed_vault_demo_data(&conn).unwrap();

        let wing_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM wings", [], |row| row.get(0))
            .unwrap();
        assert!(wing_count > 0);
    }

    #[test]
    fn upsert_project_metadata_writes_and_updates() {
        let mut conn = test_db();
        run_migrations(&mut conn).unwrap();

        let metadata = crate::models::ProjectMetadata {
            name: "Test Project".to_string(),
            project_path: "/tmp/test.grimoire".to_string(),
            database_path: ":memory:".to_string(),
            app_version: "1.0.0".to_string(),
            schema_version: SCHEMA_VERSION,
            created_at: String::new(),
            updated_at: String::new(),
        };

        upsert_project_metadata(&conn, &metadata).unwrap();

        let name: String = conn
            .query_row(
                "SELECT value FROM project_metadata WHERE key = 'name'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "Test Project");

        let updated = crate::models::ProjectMetadata {
            name: "Updated Project".to_string(),
            ..metadata
        };
        upsert_project_metadata(&conn, &updated).unwrap();

        let name: String = conn
            .query_row(
                "SELECT value FROM project_metadata WHERE key = 'name'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "Updated Project");
    }
}
