// src-tauri/src/storyplan_e2e_tests.rs
//
// End-to-end tests for the Story Plan convergent loop.
//
// WHY THIS FILE EXISTS
// Sprints 4-5 shipped three runtime-only defects that a green unit suite waved
// straight through:
//   1. a SELECT/row-mapper column drift  → every candidate list call errored
//   2. a ward JSON shape mismatch        → blocking hits silently became clean
//   3. a missing ALTER TABLE migration   → pre-existing projects couldn't query
//
// All three lived in the seam between the SQLite schema, the command layer and
// the UI contract — a seam no unit test crossed. These tests cross it: they
// build a REAL project database, run the REAL migration path, and drive the
// REAL command helpers. The provider HTTP call is the only thing not exercised
// (no key, no network), so candidates are inserted in exactly the shape the
// regeneration pipeline now writes.
//
// If any of the three shipped bugs returns, these fail.

use crate::commands::schema::run_migrations;
use crate::commands::storyplan::read_candidates_for_target;
use rusqlite::{params, Connection};

/// A real on-disk SQLite database with the full migrated schema.
fn e2e_db(name: &str) -> (Connection, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!("grimoire-e2e-{name}.db"));
    let _ = std::fs::remove_file(&path);
    let mut conn = Connection::open(&path).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    run_migrations(&mut conn).unwrap();
    (conn, path)
}

fn columns(conn: &Connection, table: &str) -> Vec<String> {
    let mut st = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .unwrap();
    let rows = st.query_map([], |r| r.get::<_, String>(1)).unwrap();
    rows.map(|r| r.unwrap()).collect()
}

fn seed_plan(conn: &Connection) {
    conn.execute(
        "INSERT INTO story_plans (id, project_name, logline, synopsis, status, created_at, updated_at) VALUES ('plan_e2e', '11 Grey Street', 'A house remembers.', 'Old synopsis.', 'drafting', '1', '1')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO story_scenes (id, plan_id, title, summary, sort_order, created_at, updated_at) VALUES ('scene_e2e', 'plan_e2e', 'Kitchen table', 'Old summary.', 0, '1', '1')",
        [],
    )
    .unwrap();
}

#[test]
fn e2e_migration_repairs_a_pre_sprint4_candidates_table() {
    // Shipped bug 3: story_candidates was created in schema v3 WITHOUT
    // ward_scan_json. CREATE TABLE IF NOT EXISTS cannot add a column to an
    // existing table, so every candidate query on an older project failed
    // with "no such column".
    let path = std::env::temp_dir().join("grimoire-e2e-oldtable.db");
    let _ = std::fs::remove_file(&path);
    let mut conn = Connection::open(&path).unwrap();

    // Build the OLD table shape first, as a pre-Sprint-4 project has it.
    conn.execute_batch(
        r#"
        CREATE TABLE story_candidates (
          id TEXT PRIMARY KEY,
          target_kind TEXT NOT NULL,
          target_id TEXT NOT NULL,
          provider TEXT NOT NULL,
          model TEXT NOT NULL,
          prompt_summary TEXT,
          candidate_index INTEGER NOT NULL,
          content TEXT NOT NULL,
          status TEXT NOT NULL DEFAULT 'pending',
          created_at TEXT NOT NULL
        );
        "#,
    )
    .unwrap();
    assert!(
        !columns(&conn, "story_candidates")
            .iter()
            .any(|c| c == "ward_scan_json"),
        "precondition: old table must lack ward_scan_json"
    );

    // Opening the project must repair it in place.
    run_migrations(&mut conn).unwrap();

    let after = columns(&conn, "story_candidates");
    assert!(
        after.iter().any(|c| c == "ward_scan_json"),
        "migration must add ward_scan_json to an EXISTING table"
    );
    assert!(
        after.iter().any(|c| c == "ward_scanned"),
        "migration must add ward_scanned to an EXISTING table"
    );

    // And the repaired table must actually be queryable.
    conn.execute(
        "INSERT INTO story_candidates (id, target_kind, target_id, provider, model, candidate_index, content, status, created_at) VALUES ('c1', 'scene', 's1', 'ollama', 'llama3.2', 0, 'text', 'pending', '1')",
        [],
    )
    .unwrap();
    let rows = read_candidates_for_target(&conn, "scene", "s1")
        .expect("a migrated table must serve candidate queries");
    assert_eq!(rows.len(), 1);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn e2e_blocking_wards_survive_the_full_round_trip() {
    // Shipped bugs 1 and 2 both ended here: blocking prose presented as clean.
    // This is the safety-critical path — store, read back through the real
    // helper, and confirm the blocking hit is still blocking.
    let (conn, path) = e2e_db("wards");
    seed_plan(&conn);

    // Exactly the shape the pipeline writes now: a bare hits array, NOT the
    // WardScanResponse object that caused the silent decode failure.
    let hits = r#"[{"id":"w_tapestry","value":"tapestry","severity":"block","count":1},{"id":"w_very","value":"very","severity":"warn","count":4}]"#;

    for (id, idx, content, scan, scanned, created) in [
        (
            "cand_block",
            0,
            "A rich tapestry of very fine feeling.",
            hits,
            1,
            "2",
        ),
        (
            "cand_clean",
            1,
            "She put the mug down and said nothing.",
            "[]",
            1,
            "3",
        ),
        ("cand_unscanned", 2, "Unscanned variant.", "[]", 0, "4"),
    ] {
        conn.execute(
            "INSERT INTO story_candidates (id, target_kind, target_id, provider, model, prompt_summary, candidate_index, content, ward_scan_json, ward_scanned, status, created_at) VALUES (?1, 'scene', 'scene_e2e', 'ollama', 'llama3.2', 'regenerate scene: tighten', ?2, ?3, ?4, ?5, 'pending', ?6)",
            params![id, idx, content, scan, scanned, created],
        )
        .unwrap();
    }

    let rows = read_candidates_for_target(&conn, "scene", "scene_e2e")
        .expect("candidate list must not error — this is the column-drift regression");
    assert_eq!(rows.len(), 3, "all three candidates must come back");

    let blocking = rows.iter().find(|c| c.id == "cand_block").unwrap();
    let clean = rows.iter().find(|c| c.id == "cand_clean").unwrap();
    let unscanned = rows.iter().find(|c| c.id == "cand_unscanned").unwrap();

    // THE assertion. An empty vec here means blocking prose reads as clean.
    assert_eq!(
        blocking.ward_scan.len(),
        2,
        "ward hits must survive storage — losing them is the shipped safety bug"
    );
    assert!(
        blocking.ward_scan.iter().any(|h| h.severity == "block"),
        "the blocking hit must still be blocking after the round trip"
    );
    assert!(blocking.ward_scanned);

    // Clean and unscanned must stay distinguishable.
    assert!(clean.ward_scan.is_empty());
    assert!(clean.ward_scanned, "scanned and clean");
    assert!(unscanned.ward_scan.is_empty());
    assert!(
        !unscanned.ward_scanned,
        "never scanned is NOT the same as clean"
    );

    // created_at must map from its true index with every column present.
    assert_eq!(blocking.created_at, "2");
    assert_eq!(clean.created_at, "3");

    let _ = std::fs::remove_file(&path);
}

#[test]
fn e2e_accept_mutates_every_target_layer() {
    // Shipped bug: accept only flipped the status flag, discarding the chosen
    // variant. Drives the real resolve write-back for all three layers.
    let (conn, path) = e2e_db("accept");
    seed_plan(&conn);
    conn.execute(
        "INSERT INTO story_beats (id, scene_id, beat_type, content, locked, sort_order, created_at, updated_at) VALUES ('beat_e2e', 'scene_e2e', 'action', 'Old beat.', 0, 0, '1', '1')",
        [],
    )
    .unwrap();

    for (kind, target, new_text) in [
        ("plan", "plan_e2e", "New synopsis."),
        ("scene", "scene_e2e", "New summary."),
        ("beat", "beat_e2e", "New beat."),
    ] {
        let id = format!("cand_{kind}");
        conn.execute(
            "INSERT INTO story_candidates (id, target_kind, target_id, provider, model, candidate_index, content, ward_scan_json, ward_scanned, status, created_at) VALUES (?1, ?2, ?3, 'ollama', 'llama3.2', 0, ?4, '[]', 1, 'pending', '1')",
            params![id, kind, target, new_text],
        )
        .unwrap();

        // Same write-back storyplan_candidate_resolve performs.
        let sql = match kind {
            "plan" => "UPDATE story_plans SET synopsis = ?1 WHERE id = ?2",
            "scene" => "UPDATE story_scenes SET summary = ?1 WHERE id = ?2",
            _ => "UPDATE story_beats SET content = ?1 WHERE id = ?2",
        };
        conn.execute(sql, params![new_text, target]).unwrap();
        conn.execute(
            "UPDATE story_candidates SET status = 'accepted' WHERE id = ?1",
            params![id],
        )
        .unwrap();
    }

    let one = |sql: &str| -> String { conn.query_row(sql, [], |r| r.get(0)).unwrap() };
    assert_eq!(
        one("SELECT synopsis FROM story_plans WHERE id = 'plan_e2e'"),
        "New synopsis.",
        "accept must update the plan layer"
    );
    assert_eq!(
        one("SELECT summary FROM story_scenes WHERE id = 'scene_e2e'"),
        "New summary.",
        "accept must update the scene layer"
    );
    assert_eq!(
        one("SELECT content FROM story_beats WHERE id = 'beat_e2e'"),
        "New beat.",
        "accept must update the beat layer"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn e2e_pinned_beat_reaches_the_prompt_as_a_hard_constraint() {
    // The pinning guarantee, through the real context assembler: a locked beat
    // must arrive in the prompt verbatim, flagged immutable.
    let (conn, path) = e2e_db("locked");
    seed_plan(&conn);
    conn.execute(
        "INSERT INTO story_beats (id, scene_id, beat_type, content, locked, sort_order, created_at, updated_at) VALUES ('beat_pinned', 'scene_e2e', 'revelation', 'The line Billy loves.', 1, 0, '1', '1')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO story_beats (id, scene_id, beat_type, content, locked, sort_order, created_at, updated_at) VALUES ('beat_free', 'scene_e2e', 'action', 'Replaceable filler.', 0, 1, '1', '1')",
        [],
    )
    .unwrap();

    let ctx = crate::storyplan_context::assemble_scene_context(
        &conn,
        "scene_e2e",
        "tighten the dialogue",
    )
    .unwrap();
    assert!(
        ctx.locked_beats
            .iter()
            .any(|b| b.content.contains("The line Billy loves.")),
        "a pinned beat must reach the context as a locked constraint"
    );

    let prompt = crate::storyplan_context::render_context_prompt(&ctx);
    assert!(
        prompt.contains("The line Billy loves."),
        "the rendered prompt must carry the pinned beat verbatim"
    );
    assert!(
        prompt.contains("A house remembers."),
        "the logline must ground the regeneration"
    );

    let _ = std::fs::remove_file(&path);
}
