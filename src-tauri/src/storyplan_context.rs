//! Story Plan regeneration context assembly.
//!
//! The core Fabula trick, rebuilt for Grimoire: when regenerating a scene (or
//! any plan layer), the prompt is assembled from six deterministic sources so
//! iteration converges instead of drifting:
//!
//! 1. Logline + synopsis (from `story_plans`)
//! 2. Character facts for characters present in the target scene (FTS5
//!    retrieval against Vault `character` items — the same grounding path
//!    `chat_with_vault` uses)
//! 3. Final beat of the previous scene (continuity entry point)
//! 4. Opening beat of the next scene (landing constraint)
//! 5. All locked beats in the target scene (must-not-change)
//! 6. The writer's edit instruction
//!
//! Everything here is pure SQLite reads + string assembly — no network, no
//! provider calls — so the assembler is fully unit-testable.

use crate::errors::CommandResult;
use crate::models::{SearchChunkResult, StoryPlan, StoryScene};
use rusqlite::{params, Connection};
use serde::Serialize;

/// How many Vault character-fact excerpts to surface per character.
const CHARACTER_FACT_LIMIT: i64 = 2;
/// Hard cap on total character facts, keeping scene-level context lean.
const CHARACTER_FACT_TOTAL_CAP: usize = 6;
/// Snippet length cap — regeneration context favours breadth over one giant excerpt.
const CHARACTER_SNIPPET_MAX_CHARS: usize = 600;

/// The assembled regeneration context for one target. Serialized into the
/// response so the UI (and tests) can inspect exactly what the model saw.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegenerationContext {
    pub plan_name: String,
    pub logline: Option<String>,
    pub synopsis: Option<String>,
    /// Scene the regeneration targets (for scene/beat/script targets).
    pub scene: Option<StoryScene>,
    /// Character facts retrieved from the Vault for this scene's characters.
    pub character_facts: Vec<CharacterFact>,
    /// Final beat of the previous scene (continuity entry point).
    pub previous_scene_anchor: Option<BeatAnchor>,
    /// Opening beat of the next scene (landing constraint).
    pub next_scene_anchor: Option<BeatAnchor>,
    /// Locked beats inside the target scene — hard constraints.
    pub locked_beats: Vec<BeatAnchor>,
    /// Scene outline — populated for plan-level regeneration only.
    pub scene_outline: Vec<SceneOutlineEntry>,
    /// The writer's edit instruction, echoed for traceability.
    pub instruction: String,
}

/// One line of the plan outline used by plan-level regeneration.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneOutlineEntry {
    pub title: String,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterFact {
    pub character: String,
    pub source_title: String,
    pub source_path: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeatAnchor {
    pub scene_title: String,
    pub beat_type: String,
    pub content: String,
}

/// Read a single scene row by id.
fn read_scene(connection: &Connection, scene_id: &str) -> CommandResult<StoryScene> {
    connection
        .query_row(
            r#"
            SELECT id, plan_id, title, setting, summary, linked_item_id, sort_order, created_at, updated_at
            FROM story_scenes WHERE id = ?1
            "#,
            params![scene_id],
            |row| {
                Ok(StoryScene {
                    id: row.get(0)?,
                    plan_id: row.get(1)?,
                    title: row.get(2)?,
                    setting: row.get(3)?,
                    summary: row.get(4)?,
                    linked_item_id: row.get(5)?,
                    sort_order: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            },
        )
        .map_err(|_| "Could not find that story scene.".to_string())
}

/// Read the plan a scene belongs to.
fn read_plan_for_scene(connection: &Connection, scene_id: &str) -> CommandResult<StoryPlan> {
    let plan_id: String = connection
        .query_row(
            "SELECT plan_id FROM story_scenes WHERE id = ?1",
            params![scene_id],
            |row| row.get(0),
        )
        .map_err(|_| "Could not find that story scene.".to_string())?;
    connection
        .query_row(
            r#"
            SELECT id, project_name, logline, synopsis, status, created_at, updated_at
            FROM story_plans WHERE id = ?1
            "#,
            params![plan_id],
            |row| {
                Ok(StoryPlan {
                    id: row.get(0)?,
                    project_name: row.get(1)?,
                    logline: row.get(2)?,
                    synopsis: row.get(3)?,
                    status: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .map_err(|_| "Could not find that story plan.".to_string())
}

/// Character names involved in a scene: the union of beat `characters` lists.
fn scene_character_names(connection: &Connection, scene_id: &str) -> CommandResult<Vec<String>> {
    let mut statement = connection
        .prepare("SELECT characters FROM story_beats WHERE scene_id = ?1")
        .map_err(|error| format!("Could not read story beats: {error}"))?;
    let rows = statement
        .query_map(params![scene_id], |row| {
            row.get::<_, Option<String>>(0)
        })
        .map_err(|error| format!("Could not read story beats: {error}"))?;

    let mut names: Vec<String> = Vec::new();
    for row in rows {
        let raw = row.map_err(|error| format!("Could not read story beats: {error}"))?;
        if let Some(json) = raw {
            if let Ok(parsed) = serde_json::from_str::<Vec<String>>(&json) {
                for name in parsed {
                    let trimmed = name.trim().to_string();
                    if !trimmed.is_empty() && !names.iter().any(|existing| existing.eq_ignore_ascii_case(&trimmed)) {
                        names.push(trimmed);
                    }
                }
            }
        }
    }
    Ok(names)
}

/// FTS5 retrieval of Vault `character` items for a set of names — the same
/// grounding machinery `chat_with_vault` uses, but with the character filter
/// applied in the query itself so manuscript chunks can't crowd out sheets
/// (Codex catch on PR #25).
fn retrieve_character_facts(
    connection: &Connection,
    names: &[String],
) -> CommandResult<Vec<CharacterFact>> {
    let mut facts: Vec<CharacterFact> = Vec::new();
    for name in names {
        if facts.len() >= CHARACTER_FACT_TOTAL_CAP {
            break;
        }
        let results =
            crate::db::search_character_chunks_internal(connection, name, CHARACTER_FACT_LIMIT)?;
        let mut taken_for_character = 0i64;
        for result in results {
            if taken_for_character >= CHARACTER_FACT_LIMIT || facts.len() >= CHARACTER_FACT_TOTAL_CAP {
                break;
            }
            // Skip facts that duplicate an already-captured source chunk.
            if facts
                .iter()
                .any(|existing| existing.source_title == result.title && existing.snippet == truncate_snippet(&result.snippet))
            {
                continue;
            }
            facts.push(character_fact_from_search(name, result));
            taken_for_character += 1;
        }
    }
    Ok(facts)
}

fn truncate_snippet(snippet: &str) -> String {
    let trimmed = snippet.trim();
    if trimmed.chars().count() <= CHARACTER_SNIPPET_MAX_CHARS {
        trimmed.to_string()
    } else {
        let cut: String = trimmed.chars().take(CHARACTER_SNIPPET_MAX_CHARS).collect();
        format!("{cut}…")
    }
}

fn character_fact_from_search(character: &str, result: SearchChunkResult) -> CharacterFact {
    CharacterFact {
        character: character.to_string(),
        source_title: result.title,
        source_path: result.vault_path,
        snippet: truncate_snippet(&result.snippet),
    }
}

/// The final beat of the scene immediately before `scene` in sort order.
fn previous_scene_anchor(
    connection: &Connection,
    plan_id: &str,
    scene_sort_order: i64,
) -> CommandResult<Option<BeatAnchor>> {
    let previous: Option<(String, String)> = connection
        .query_row(
            r#"
            SELECT id, title FROM story_scenes
            WHERE plan_id = ?1 AND sort_order < ?2
            ORDER BY sort_order DESC LIMIT 1
            "#,
            params![plan_id, scene_sort_order],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();
    let Some((previous_id, previous_title)) = previous else {
        return Ok(None);
    };
    beat_anchor_for(connection, &previous_id, &previous_title, "last")
}

/// The opening beat of the scene immediately after `scene` in sort order.
fn next_scene_anchor(
    connection: &Connection,
    plan_id: &str,
    scene_sort_order: i64,
) -> CommandResult<Option<BeatAnchor>> {
    let next: Option<(String, String)> = connection
        .query_row(
            r#"
            SELECT id, title FROM story_scenes
            WHERE plan_id = ?1 AND sort_order > ?2
            ORDER BY sort_order ASC LIMIT 1
            "#,
            params![plan_id, scene_sort_order],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();
    let Some((next_id, next_title)) = next else {
        return Ok(None);
    };
    beat_anchor_for(connection, &next_id, &next_title, "first")
}

fn beat_anchor_for(
    connection: &Connection,
    scene_id: &str,
    scene_title: &str,
    position: &str,
) -> CommandResult<Option<BeatAnchor>> {
    let order = if position == "last" { "DESC" } else { "ASC" };
    let row = connection.query_row(
        &format!(
            "SELECT beat_type, content FROM story_beats WHERE scene_id = ?1 ORDER BY sort_order {order} LIMIT 1"
        ),
        params![scene_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    );
    match row {
        Ok((beat_type, content)) => Ok(Some(BeatAnchor {
            scene_title: scene_title.to_string(),
            beat_type,
            content,
        })),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(format!("Could not read anchoring beats: {error}")),
    }
}

/// All locked beats inside the target scene, in story order.
fn locked_beats_for(connection: &Connection, scene_id: &str, scene_title: &str) -> CommandResult<Vec<BeatAnchor>> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT beat_type, content FROM story_beats
            WHERE scene_id = ?1 AND locked = 1
            ORDER BY sort_order ASC
            "#,
        )
        .map_err(|error| format!("Could not read locked beats: {error}"))?;
    let rows = statement
        .query_map(params![scene_id], |row| {
            Ok(BeatAnchor {
                scene_title: scene_title.to_string(),
                beat_type: row.get(0)?,
                content: row.get(1)?,
            })
        })
        .map_err(|error| format!("Could not read locked beats: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not read locked beats: {error}"))
}

/// Assemble the full six-point regeneration context for a scene target.
///
/// `scene_id` may target a scene directly, or be resolved by the caller for
/// beat targets. The instruction is the writer's edit note (point 6).
pub fn assemble_scene_context(
    connection: &Connection,
    scene_id: &str,
    instruction: &str,
) -> CommandResult<RegenerationContext> {
    let scene = read_scene(connection, scene_id)?;
    let plan = read_plan_for_scene(connection, scene_id)?;
    let names = scene_character_names(connection, scene_id)?;
    let character_facts = retrieve_character_facts(connection, &names)?;

    Ok(RegenerationContext {
        plan_name: plan.project_name,
        logline: plan.logline,
        synopsis: plan.synopsis,
        previous_scene_anchor: previous_scene_anchor(connection, &plan.id, scene.sort_order)?,
        next_scene_anchor: next_scene_anchor(connection, &plan.id, scene.sort_order)?,
        locked_beats: locked_beats_for(connection, scene_id, &scene.title)?,
        scene: Some(scene),
        character_facts,
        scene_outline: Vec::new(),
        instruction: instruction.trim().to_string(),
    })
}

/// Context for plan-level regeneration: plan identity plus the full scene
/// outline (title + summary per scene). Locked beats across the whole plan
/// ride along as constraints — a regenerated outline must respect them.
pub fn assemble_plan_context(
    connection: &Connection,
    plan_id: &str,
    instruction: &str,
) -> CommandResult<RegenerationContext> {
    let plan: StoryPlan = connection
        .query_row(
            r#"
            SELECT id, project_name, logline, synopsis, status, created_at, updated_at
            FROM story_plans WHERE id = ?1
            "#,
            params![plan_id],
            |row| {
                Ok(StoryPlan {
                    id: row.get(0)?,
                    project_name: row.get(1)?,
                    logline: row.get(2)?,
                    synopsis: row.get(3)?,
                    status: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .map_err(|_| "Could not find that story plan.".to_string())?;

    let mut statement = connection
        .prepare(
            r#"
            SELECT id, title, summary FROM story_scenes
            WHERE plan_id = ?1 ORDER BY sort_order ASC
            "#,
        )
        .map_err(|error| format!("Could not read story scenes: {error}"))?;
    let rows = statement
        .query_map(params![plan_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|error| format!("Could not read story scenes: {error}"))?;

    let mut scene_outline: Vec<SceneOutlineEntry> = Vec::new();
    let mut locked_beats: Vec<BeatAnchor> = Vec::new();
    for row in rows {
        let (scene_id, title, summary) =
            row.map_err(|error| format!("Could not read story scenes: {error}"))?;
        locked_beats.extend(locked_beats_for(connection, &scene_id, &title)?);
        scene_outline.push(SceneOutlineEntry { title, summary });
    }

    Ok(RegenerationContext {
        plan_name: plan.project_name,
        logline: plan.logline,
        synopsis: plan.synopsis,
        scene: None,
        character_facts: Vec::new(),
        previous_scene_anchor: None,
        next_scene_anchor: None,
        locked_beats,
        scene_outline,
        instruction: instruction.trim().to_string(),
    })
}

/// Render the assembled context as the user-prompt body sent to the model.
/// Deterministic ordering; empty sections are omitted so small plans don't
/// carry noise. Locked beats are rendered as explicit hard constraints.
pub fn render_context_prompt(context: &RegenerationContext) -> String {
    let mut sections: Vec<String> = Vec::new();

    sections.push(format!("Story plan: {}", context.plan_name));
    if let Some(logline) = context.logline.as_deref().filter(|value| !value.trim().is_empty()) {
        sections.push(format!("Logline:\n{logline}"));
    }
    if let Some(synopsis) = context.synopsis.as_deref().filter(|value| !value.trim().is_empty()) {
        sections.push(format!("Synopsis:\n{synopsis}"));
    }

    if let Some(previous) = &context.previous_scene_anchor {
        sections.push(format!(
            "Continuity — final beat of the previous scene (\"{}\"):\n[{}] {}",
            previous.scene_title, previous.beat_type, previous.content
        ));
    }
    if let Some(next) = &context.next_scene_anchor {
        sections.push(format!(
            "Landing constraint — opening beat of the next scene (\"{}\"):\n[{}] {}",
            next.scene_title, next.beat_type, next.content
        ));
    }

    if !context.locked_beats.is_empty() {
        let mut locked = String::from(
            "Locked beats — these are final. Keep them verbatim in meaning and never rewrite them:",
        );
        for beat in &context.locked_beats {
            // Plan-level regeneration spans many scenes, so each constraint
            // must name the scene it belongs to or the model can attach a
            // locked event to the wrong outline scene (Codex catch on PR #25).
            if context.scene.is_none() {
                locked.push_str(&format!(
                    "\n- [{}] (in scene \"{}\") {}",
                    beat.beat_type, beat.scene_title, beat.content
                ));
            } else {
                locked.push_str(&format!("\n- [{}] {}", beat.beat_type, beat.content));
            }
        }
        sections.push(locked);
    }

    if !context.character_facts.is_empty() {
        let mut facts = String::from("Character facts from the Vault:");
        for fact in &context.character_facts {
            facts.push_str(&format!(
                "\n- {} (from \"{}\"):\n  {}",
                fact.character, fact.source_title, fact.snippet
            ));
        }
        sections.push(facts);
    }

    if let Some(scene) = &context.scene {
        let mut scene_section = format!("Scene to regenerate: \"{}\"", scene.title);
        if let Some(setting) = scene.setting.as_deref().filter(|value| !value.trim().is_empty()) {
            scene_section.push_str(&format!("\nSetting: {setting}"));
        }
        if let Some(summary) = scene.summary.as_deref().filter(|value| !value.trim().is_empty()) {
            scene_section.push_str(&format!("\nCurrent summary: {summary}"));
        }
        sections.push(scene_section);
    }

    if !context.scene_outline.is_empty() {
        let mut outline = String::from("Current scene outline (in story order):");
        for (index, entry) in context.scene_outline.iter().enumerate() {
            let summary = entry
                .summary
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(|value| format!(" — {value}"))
                .unwrap_or_default();
            outline.push_str(&format!("\n{}. {}{}", index + 1, entry.title, summary));
        }
        sections.push(outline);
    }

    sections.join("\n\n")
}

/// The system prompt for regeneration. Locked beats are named again here as
/// hard constraints — the writer's word is final, same philosophy as wards.
pub fn regeneration_system_prompt(context: &RegenerationContext) -> String {
    let mut prompt = String::from(
        "You are the story engine inside Grimoire, a local-first writing studio. \
Regenerate only the requested story layer. Keep the output aligned with the \
provided story plan, character facts, and adjacent-scene anchors. \
Write in the voice and tense already established by the surrounding beats. \
Return only the regenerated content — no preamble, no meta commentary.",
    );
    if !context.locked_beats.is_empty() {
        prompt.push_str(
            " Locked beats are warded content: preserve their meaning exactly, \
treat them as immovable, and never propose rewrites of them.",
        );
    }
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::schema::run_migrations;

    fn test_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        run_migrations(&mut conn).unwrap();
        conn
    }

    fn insert_plan_with(conn: &Connection, plan_id: &str, logline: Option<&str>, synopsis: Option<&str>) {
        conn.execute(
            "INSERT INTO story_plans (id, project_name, logline, synopsis, status, created_at, updated_at) VALUES (?1, 'Eleven Grey Street', ?2, ?3, 'drafting', '1', '1')",
            params![plan_id, logline, synopsis],
        )
        .unwrap();
    }

    fn insert_scene_with(
        conn: &Connection,
        scene_id: &str,
        plan_id: &str,
        title: &str,
        sort_order: i64,
        setting: Option<&str>,
        summary: Option<&str>,
    ) {
        conn.execute(
            "INSERT INTO story_scenes (id, plan_id, title, setting, summary, sort_order, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '1', '1')",
            params![scene_id, plan_id, title, setting, summary, sort_order],
        )
        .unwrap();
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_beat_with(
        conn: &Connection,
        beat_id: &str,
        scene_id: &str,
        beat_type: &str,
        content: &str,
        characters: Option<&str>,
        locked: i64,
        sort_order: i64,
    ) {
        conn.execute(
            "INSERT INTO story_beats (id, scene_id, beat_type, content, characters, locked, sort_order, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, '1', '1')",
            params![beat_id, scene_id, beat_type, content, characters, locked, sort_order],
        )
        .unwrap();
    }

    fn seed_fts_character_fact(conn: &Connection, item_id: &str, title: &str, text: &str) {
        // Items carry FKs through the whole vault hierarchy — build it once.
        conn.execute(
            "INSERT OR IGNORE INTO wings (id, name, description, sort_order, created_at, updated_at) VALUES ('w_test', 'Characters', NULL, 0, '', '')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO halls (id, wing_id, name, description, sort_order, created_at, updated_at) VALUES ('h_test', 'w_test', 'Cast', NULL, 0, '', '')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO rooms (id, hall_id, name, description, sort_order, created_at, updated_at) VALUES ('r_test', 'h_test', 'Main', NULL, 0, '', '')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO drawers (id, room_id, name, description, sort_order, created_at, updated_at) VALUES ('d_test', 'r_test', 'Leads', NULL, 0, '', '')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO items (id, drawer_id, title, item_type, content, plain_text, word_count, memory_enabled, source_kind, sort_order, created_at, updated_at) VALUES (?1, 'd_test', ?2, 'character', ?3, ?3, 10, 1, 'manual', 0, '', '')",
            params![item_id, title, text],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO item_chunks_fts (chunk_id, item_id, title, item_type, vault_path, text) VALUES (?1, ?2, ?3, 'character', 'Characters / Cast / Main / Leads / ?3', ?4)",
            params![format!("{item_id}_chunk_0"), item_id, title, text],
        )
        .unwrap();
    }

    #[test]
    fn assembles_six_point_context_for_middle_scene() {
        let conn = test_db();
        insert_plan_with(
            &conn,
            "plan_1",
            Some("A debt collector finds her own name in the ledger."),
            Some("Mara works the grey streets collecting what is owed."),
        );
        insert_scene_with(&conn, "scene_a", "plan_1", "The Ledger", 0, None, None);
        insert_scene_with(
            &conn,
            "scene_b",
            "plan_1",
            "Kitchen Debt",
            1,
            Some("Mara's kitchen, night"),
            Some("Jonah confronts Mara about the ledger."),
        );
        insert_scene_with(&conn, "scene_c", "plan_1", "The Drop", 2, None, None);

        // Previous scene ends on a revelation; next scene opens on action.
        insert_beat_with(&conn, "beat_a1", "scene_a", "revelation", "The ledger lists Mara herself.", None, 0, 0);
        // Target scene: locked beat + character roster.
        insert_beat_with(
            &conn,
            "beat_b1",
            "scene_b",
            "dialogue",
            "You knew about the ledger, Mara.",
            Some(r#"["Mara","Jonah"]"#),
            0,
            0,
        );
        insert_beat_with(
            &conn,
            "beat_b2",
            "scene_b",
            "action",
            "Mara slides the ledger across the table.",
            Some(r#"["Mara"]"#),
            1,
            1,
        );
        insert_beat_with(&conn, "beat_c1", "scene_c", "action", "Jonah waits by the canal.", None, 0, 0);

        let context = assemble_scene_context(&conn, "scene_b", "make the confrontation colder").unwrap();

        // Points 1-2: plan identity.
        assert_eq!(context.plan_name, "Eleven Grey Street");
        assert!(context.logline.unwrap().contains("ledger"));
        // Point 3: continuity anchor from the previous scene.
        let previous = context.previous_scene_anchor.unwrap();
        assert_eq!(previous.scene_title, "The Ledger");
        assert!(previous.content.contains("lists Mara"));
        // Point 4: landing constraint from the next scene.
        let next = context.next_scene_anchor.unwrap();
        assert_eq!(next.scene_title, "The Drop");
        assert!(next.content.contains("canal"));
        // Point 5: locked beats only — the unlocked dialogue beat must not appear.
        assert_eq!(context.locked_beats.len(), 1);
        assert!(context.locked_beats[0].content.contains("slides the ledger"));
        // Point 6: instruction echoed.
        assert_eq!(context.instruction, "make the confrontation colder");
        // Scene metadata carried through.
        let scene = context.scene.unwrap();
        assert_eq!(scene.title, "Kitchen Debt");
        assert_eq!(scene.setting.as_deref(), Some("Mara's kitchen, night"));
    }

    #[test]
    fn locked_beat_excluded_from_rewrite_surface_but_present_as_constraint() {
        let conn = test_db();
        insert_plan_with(&conn, "plan_1", None, None);
        insert_scene_with(&conn, "scene_1", "plan_1", "Only Scene", 0, None, None);
        insert_beat_with(&conn, "beat_locked", "scene_1", "revelation", "The vault was empty all along.", None, 1, 0);
        insert_beat_with(&conn, "beat_free", "scene_1", "action", "Someone knocks twice.", None, 0, 1);

        let context = assemble_scene_context(&conn, "scene_1", "tighten").unwrap();
        let prompt = render_context_prompt(&context);

        // Locked beat rides in the prompt as a hard constraint…
        assert!(prompt.contains("Locked beats"));
        assert!(prompt.contains("The vault was empty all along."));
        // …while unlocked beats are not surfaced as protected content.
        assert!(!prompt.contains("Someone knocks twice."));
    }

    #[test]
    fn character_facts_retrieved_from_vault_fts() {
        let conn = test_db();
        insert_plan_with(&conn, "plan_1", None, None);
        insert_scene_with(&conn, "scene_1", "plan_1", "Mara Scene", 0, None, None);
        insert_beat_with(&conn, "beat_1", "scene_1", "action", "Mara counts the cash.", Some(r#"["Mara"]"#), 0, 0);
        seed_fts_character_fact(
            &conn,
            "char_mara",
            "Mara Voss",
            "Mara collects debts across Eleven Grey Street and never forgives one.",
        );

        let context = assemble_scene_context(&conn, "scene_1", "go").unwrap();
        assert_eq!(context.character_facts.len(), 1);
        assert_eq!(context.character_facts[0].character, "Mara");
        assert_eq!(context.character_facts[0].source_title, "Mara Voss");
        assert!(context.character_facts[0].snippet.contains("collects debts"));
    }

    #[test]
    fn character_names_dedupe_case_insensitively() {
        let conn = test_db();
        insert_plan_with(&conn, "plan_1", None, None);
        insert_scene_with(&conn, "scene_1", "plan_1", "Scene", 0, None, None);
        insert_beat_with(&conn, "beat_1", "scene_1", "dialogue", "a", Some(r#"["Mara"]"#), 0, 0);
        insert_beat_with(&conn, "beat_2", "scene_1", "dialogue", "b", Some(r#"["mara","Jonah"]"#), 0, 1);

        let names = scene_character_names(&conn, "scene_1").unwrap();
        assert_eq!(names, vec!["Mara".to_string(), "Jonah".to_string()]);
    }

    #[test]
    fn edge_scene_has_no_anchors() {
        let conn = test_db();
        insert_plan_with(&conn, "plan_1", None, None);
        insert_scene_with(&conn, "scene_1", "plan_1", "First", 0, None, None);

        let context = assemble_scene_context(&conn, "scene_1", "open strong").unwrap();
        assert!(context.previous_scene_anchor.is_none());
        assert!(context.next_scene_anchor.is_none());
    }

    #[test]
    fn render_prompt_omits_empty_sections() {
        let conn = test_db();
        insert_plan_with(&conn, "plan_1", None, None);
        insert_scene_with(&conn, "scene_1", "plan_1", "Bare Scene", 0, None, None);

        let context = assemble_scene_context(&conn, "scene_1", "fill it in").unwrap();
        let prompt = render_context_prompt(&context);

        assert!(prompt.contains("Story plan: Eleven Grey Street"));
        assert!(prompt.contains("Scene to regenerate: \"Bare Scene\""));
        assert!(!prompt.contains("Logline:"));
        assert!(!prompt.contains("Synopsis:"));
        assert!(!prompt.contains("Locked beats"));
        assert!(!prompt.contains("Character facts"));
    }

    #[test]
    fn system_prompt_names_wards_only_when_locked_beats_exist() {
        let conn = test_db();
        insert_plan_with(&conn, "plan_1", None, None);
        insert_scene_with(&conn, "scene_1", "plan_1", "Scene", 0, None, None);
        insert_beat_with(&conn, "beat_1", "scene_1", "action", "free", None, 0, 0);

        let context = assemble_scene_context(&conn, "scene_1", "x").unwrap();
        assert!(!regeneration_system_prompt(&context).contains("warded"));

        conn.execute("UPDATE story_beats SET locked = 1 WHERE id = 'beat_1'", []).unwrap();
        let locked_context = assemble_scene_context(&conn, "scene_1", "x").unwrap();
        assert!(regeneration_system_prompt(&locked_context).contains("warded"));
    }

    #[test]
    fn missing_scene_errors_cleanly() {
        let conn = test_db();
        let result = assemble_scene_context(&conn, "scene_missing", "x");
        assert!(result.is_err());
    }

    #[test]
    fn plan_context_collects_outline_and_locked_beats_across_scenes() {
        let conn = test_db();
        insert_plan_with(&conn, "plan_1", Some("logline"), None);
        insert_scene_with(&conn, "scene_1", "plan_1", "Opening", 0, None, Some("Mara arrives."));
        insert_scene_with(&conn, "scene_2", "plan_1", "Confrontation", 1, None, None);
        insert_beat_with(&conn, "beat_1", "scene_1", "revelation", "The ledger burns.", None, 1, 0);
        insert_beat_with(&conn, "beat_2", "scene_2", "action", "A free beat.", None, 0, 0);

        let context = assemble_plan_context(&conn, "plan_1", "restructure act two").unwrap();
        assert_eq!(context.scene_outline.len(), 2);
        assert_eq!(context.scene_outline[0].title, "Opening");
        assert_eq!(context.scene_outline[0].summary.as_deref(), Some("Mara arrives."));
        // Only the locked beat rides along as a constraint.
        assert_eq!(context.locked_beats.len(), 1);
        assert!(context.locked_beats[0].content.contains("ledger burns"));
        assert!(context.scene.is_none());

        let prompt = render_context_prompt(&context);
        assert!(prompt.contains("Current scene outline"));
        assert!(prompt.contains("1. Opening — Mara arrives."));
        assert!(prompt.contains("2. Confrontation"));
    }

    #[test]
    fn truncate_snippet_caps_long_excerpts() {
        let long = "word ".repeat(300);
        let truncated = truncate_snippet(&long);
        assert!(truncated.chars().count() <= CHARACTER_SNIPPET_MAX_CHARS + 1);
        assert!(truncated.ends_with('…'));
    }
}
