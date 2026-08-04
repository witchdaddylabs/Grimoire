use super::*;
use crate::db::next_sort_order;
use crate::helpers::{timestamp, timestamp_nanos};

const VALID_BEAT_TYPES: [&str; 6] = [
    "action",
    "dialogue",
    "revelation",
    "conflict",
    "transition",
    "other",
];

const VALID_PLAN_STATUSES: [&str; 5] = ["draft", "outline", "drafting", "revision", "done"];

// ── Read helpers ──

pub(crate) fn read_plan(connection: &Connection, plan_id: &str) -> CommandResult<StoryPlan> {
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

pub(crate) fn read_scenes(connection: &Connection, plan_id: &str) -> CommandResult<Vec<StoryScene>> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT id, plan_id, title, setting, summary, linked_item_id, sort_order, created_at, updated_at
            FROM story_scenes WHERE plan_id = ?1 ORDER BY sort_order ASC
            "#,
        )
        .map_err(|error| format!("Could not read story scenes: {error}"))?;

    let rows = statement
        .query_map(params![plan_id], |row| {
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
        })
        .map_err(|error| format!("Could not read story scenes: {error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not read story scenes: {error}"))
}

pub(crate) fn read_beats(connection: &Connection, scene_id: &str) -> CommandResult<Vec<StoryBeat>> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT id, scene_id, beat_type, content, characters, locked, sort_order, created_at, updated_at
            FROM story_beats WHERE scene_id = ?1 ORDER BY sort_order ASC
            "#,
        )
        .map_err(|error| format!("Could not read story beats: {error}"))?;

    let rows = statement
        .query_map(params![scene_id], |row| {
            let characters_json: Option<String> = row.get(4)?;
            let locked_int: i64 = row.get(5)?;
            Ok(StoryBeat {
                id: row.get(0)?,
                scene_id: row.get(1)?,
                beat_type: row.get(2)?,
                content: row.get(3)?,
                characters: characters_json
                    .as_deref()
                    .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok()),
                locked: locked_int != 0,
                sort_order: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .map_err(|error| format!("Could not read story beats: {error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not read story beats: {error}"))
}

pub(crate) fn read_plan_detail(connection: &Connection, plan_id: &str) -> CommandResult<StoryPlanDetail> {
    let plan = read_plan(connection, plan_id)?;
    let scenes = read_scenes(connection, plan_id)?;
    let mut scenes_with_beats = Vec::with_capacity(scenes.len());
    for scene in scenes {
        let beats = read_beats(connection, &scene.id)?;
        scenes_with_beats.push(StorySceneWithBeats { scene, beats });
    }
    Ok(StoryPlanDetail {
        plan,
        scenes: scenes_with_beats,
    })
}

fn plan_id_for_scene(connection: &Connection, scene_id: &str) -> CommandResult<String> {
    connection
        .query_row(
            "SELECT plan_id FROM story_scenes WHERE id = ?1",
            params![scene_id],
            |row| row.get(0),
        )
        .map_err(|_| "Could not find that story scene.".to_string())
}

fn scene_id_for_beat(connection: &Connection, beat_id: &str) -> CommandResult<String> {
    connection
        .query_row(
            "SELECT scene_id FROM story_beats WHERE id = ?1",
            params![beat_id],
            |row| row.get(0),
        )
        .map_err(|_| "Could not find that story beat.".to_string())
}

fn characters_to_json(characters: Option<Vec<String>>) -> CommandResult<Option<String>> {
    match characters {
        Some(names) => {
            let trimmed: Vec<String> = names
                .into_iter()
                .map(|name| name.trim().to_string())
                .filter(|name| !name.is_empty())
                .collect();
            if trimmed.is_empty() {
                return Ok(None);
            }
            serde_json::to_string(&trimmed)
                .map(Some)
                .map_err(|error| format!("Could not encode beat characters: {error}"))
        }
        None => Ok(None),
    }
}

fn validate_beat_type(beat_type: &str) -> CommandResult<()> {
    if VALID_BEAT_TYPES.contains(&beat_type) {
        Ok(())
    } else {
        Err(format!(
            "Invalid beat type. Choose one of: {}",
            VALID_BEAT_TYPES.join(", ")
        ))
    }
}

fn touch_plan(connection: &Connection, plan_id: &str) -> CommandResult<()> {
    connection
        .execute(
            "UPDATE story_plans SET updated_at = ?1 WHERE id = ?2",
            params![timestamp(), plan_id],
        )
        .map_err(|error| format!("Could not update story plan timestamp: {error}"))?;
    Ok(())
}

/// Candidates are keyed by polymorphic target, so FK cascades cannot clean
/// them up. Remove any candidates targeting the given ids.
fn delete_candidates_for(connection: &Connection, target_ids: &[String]) -> CommandResult<()> {
    for target_id in target_ids {
        connection
            .execute(
                "DELETE FROM story_candidates WHERE target_id = ?1",
                params![target_id],
            )
            .map_err(|error| format!("Could not clean up story candidates: {error}"))?;
    }
    Ok(())
}

// ── Plan commands ──

#[tauri::command]
pub fn storyplan_list(project_path: String) -> CommandResult<StoryPlanListResponse> {
    let connection = open_project_database(&project_path)?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT id, project_name, logline, synopsis, status, created_at, updated_at
            FROM story_plans ORDER BY updated_at DESC
            "#,
        )
        .map_err(|error| format!("Could not read story plans: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            Ok(StoryPlan {
                id: row.get(0)?,
                project_name: row.get(1)?,
                logline: row.get(2)?,
                synopsis: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|error| format!("Could not read story plans: {error}"))?;

    let plans = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not read story plans: {error}"))?;

    Ok(StoryPlanListResponse { plans })
}

#[tauri::command]
pub fn storyplan_create(request: StoryPlanCreateRequest) -> CommandResult<StoryPlanDetail> {
    let connection = open_project_database(&request.project_path)?;
    let name = request.project_name.trim().to_string();
    if name.is_empty() {
        return Err("The story plan needs a name.".to_string());
    }

    let plan_id = format!("plan_{}", timestamp_nanos());
    let now = timestamp();
    connection
        .execute(
            r#"
            INSERT INTO story_plans (id, project_name, logline, synopsis, status, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, 'draft', ?5, ?5)
            "#,
            params![plan_id, name, request.logline, request.synopsis, now],
        )
        .map_err(|error| format!("Could not create story plan: {error}"))?;

    read_plan_detail(&connection, &plan_id)
}

#[tauri::command]
pub fn storyplan_get(project_path: String, plan_id: String) -> CommandResult<StoryPlanDetail> {
    let connection = open_project_database(&project_path)?;
    read_plan_detail(&connection, &plan_id)
}

#[tauri::command]
pub fn storyplan_update(request: StoryPlanUpdateRequest) -> CommandResult<StoryPlanDetail> {
    let connection = open_project_database(&request.project_path)?;
    let current = read_plan(&connection, &request.plan_id)?;

    let project_name = match request.project_name {
        Some(name) => {
            let trimmed = name.trim().to_string();
            if trimmed.is_empty() {
                return Err("The story plan needs a name.".to_string());
            }
            trimmed
        }
        None => current.project_name,
    };

    let status = match request.status {
        Some(status) => {
            let trimmed = status.trim().to_lowercase();
            if !VALID_PLAN_STATUSES.contains(&trimmed.as_str()) {
                return Err(format!(
                    "Invalid plan status. Choose one of: {}",
                    VALID_PLAN_STATUSES.join(", ")
                ));
            }
            trimmed
        }
        None => current.status,
    };

    // Explicit empty strings clear the field; None leaves it untouched.
    let logline = match request.logline {
        Some(value) => {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        }
        None => current.logline,
    };
    let synopsis = match request.synopsis {
        Some(value) => {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        }
        None => current.synopsis,
    };

    connection
        .execute(
            r#"
            UPDATE story_plans
            SET project_name = ?1, logline = ?2, synopsis = ?3, status = ?4, updated_at = ?5
            WHERE id = ?6
            "#,
            params![project_name, logline, synopsis, status, timestamp(), request.plan_id],
        )
        .map_err(|error| format!("Could not update story plan: {error}"))?;

    read_plan_detail(&connection, &request.plan_id)
}

#[tauri::command]
pub fn storyplan_delete(request: StoryPlanDeleteRequest) -> CommandResult<StoryPlanListResponse> {
    let connection = open_project_database(&request.project_path)?;

    // Candidates are polymorphic (no FK), so collect dependent ids first.
    let mut target_ids: Vec<String> = vec![request.plan_id.clone()];
    for scene in read_scenes(&connection, &request.plan_id)? {
        target_ids.extend(read_beats(&connection, &scene.id)?.into_iter().map(|beat| beat.id));
        target_ids.push(scene.id);
    }

    connection
        .execute(
            "DELETE FROM story_plans WHERE id = ?1",
            params![request.plan_id],
        )
        .map_err(|error| format!("Could not delete story plan: {error}"))?;

    if connection.changes() == 0 {
        return Err("Could not find that story plan to delete.".to_string());
    }

    delete_candidates_for(&connection, &target_ids)?;
    storyplan_list(request.project_path)
}

// ── Scene commands ──

#[tauri::command]
pub fn storyplan_scene_create(request: StorySceneCreateRequest) -> CommandResult<StoryPlanDetail> {
    let connection = open_project_database(&request.project_path)?;
    let title = request.title.trim().to_string();
    if title.is_empty() {
        return Err("A scene needs a title.".to_string());
    }
    read_plan(&connection, &request.plan_id)?;

    let scene_id = format!("scene_{}", timestamp_nanos());
    let now = timestamp();
    let sort_order = next_sort_order(&connection, "story_scenes", "plan_id", &request.plan_id)?;

    connection
        .execute(
            r#"
            INSERT INTO story_scenes (id, plan_id, title, setting, summary, linked_item_id, sort_order, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
            "#,
            params![scene_id, request.plan_id, title, request.setting, request.summary, request.linked_item_id, sort_order, now],
        )
        .map_err(|error| format!("Could not create story scene: {error}"))?;

    touch_plan(&connection, &request.plan_id)?;
    read_plan_detail(&connection, &request.plan_id)
}

#[tauri::command]
pub fn storyplan_scene_update(request: StorySceneUpdateRequest) -> CommandResult<StoryPlanDetail> {
    let connection = open_project_database(&request.project_path)?;
    let plan_id = plan_id_for_scene(&connection, &request.scene_id)?;

    let current: (String, Option<String>, Option<String>, Option<String>) = connection
        .query_row(
            "SELECT title, setting, summary, linked_item_id FROM story_scenes WHERE id = ?1",
            params![request.scene_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|_| "Could not find that story scene.".to_string())?;

    let title = match request.title {
        Some(value) => {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                return Err("A scene needs a title.".to_string());
            }
            trimmed
        }
        None => current.0,
    };

    // Empty strings clear; None leaves untouched.
    let setting = match request.setting {
        Some(value) => {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        }
        None => current.1,
    };
    let summary = match request.summary {
        Some(value) => {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        }
        None => current.2,
    };
    let linked_item_id = match request.linked_item_id {
        Some(value) => {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        }
        None => current.3,
    };

    connection
        .execute(
            r#"
            UPDATE story_scenes
            SET title = ?1, setting = ?2, summary = ?3, linked_item_id = ?4, updated_at = ?5
            WHERE id = ?6
            "#,
            params![title, setting, summary, linked_item_id, timestamp(), request.scene_id],
        )
        .map_err(|error| format!("Could not update story scene: {error}"))?;

    touch_plan(&connection, &plan_id)?;
    read_plan_detail(&connection, &plan_id)
}

#[tauri::command]
pub fn storyplan_scene_delete(request: StorySceneDeleteRequest) -> CommandResult<StoryPlanDetail> {
    let connection = open_project_database(&request.project_path)?;
    let plan_id = plan_id_for_scene(&connection, &request.scene_id)?;

    let mut target_ids: Vec<String> = vec![request.scene_id.clone()];
    target_ids.extend(
        read_beats(&connection, &request.scene_id)?
            .into_iter()
            .map(|beat| beat.id),
    );

    connection
        .execute(
            "DELETE FROM story_scenes WHERE id = ?1",
            params![request.scene_id],
        )
        .map_err(|error| format!("Could not delete story scene: {error}"))?;

    delete_candidates_for(&connection, &target_ids)?;
    touch_plan(&connection, &plan_id)?;
    read_plan_detail(&connection, &plan_id)
}

// ── Beat commands ──

#[tauri::command]
pub fn storyplan_beat_create(request: StoryBeatCreateRequest) -> CommandResult<StoryPlanDetail> {
    let connection = open_project_database(&request.project_path)?;
    let content = request.content.trim().to_string();
    if content.is_empty() {
        return Err("A beat cannot be empty.".to_string());
    }
    let beat_type = request
        .beat_type
        .as_deref()
        .unwrap_or("action")
        .trim()
        .to_lowercase();
    validate_beat_type(&beat_type)?;

    let scene_id = request.scene_id.clone();
    let plan_id = plan_id_for_scene(&connection, &scene_id)?;
    let characters = characters_to_json(request.characters)?;

    let beat_id = format!("beat_{}", timestamp_nanos());
    let now = timestamp();
    let sort_order = next_sort_order(&connection, "story_beats", "scene_id", &scene_id)?;

    connection
        .execute(
            r#"
            INSERT INTO story_beats (id, scene_id, beat_type, content, characters, locked, sort_order, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, ?7)
            "#,
            params![beat_id, scene_id, beat_type, content, characters, sort_order, now],
        )
        .map_err(|error| format!("Could not create story beat: {error}"))?;

    touch_plan(&connection, &plan_id)?;
    read_plan_detail(&connection, &plan_id)
}

#[tauri::command]
pub fn storyplan_beat_update(request: StoryBeatUpdateRequest) -> CommandResult<StoryPlanDetail> {
    let connection = open_project_database(&request.project_path)?;
    let scene_id = scene_id_for_beat(&connection, &request.beat_id)?;
    let plan_id = plan_id_for_scene(&connection, &scene_id)?;

    let current: (String, String, Option<String>) = connection
        .query_row(
            "SELECT beat_type, content, characters FROM story_beats WHERE id = ?1",
            params![request.beat_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| "Could not find that story beat.".to_string())?;

    let beat_type = match request.beat_type {
        Some(value) => {
            let trimmed = value.trim().to_lowercase();
            validate_beat_type(&trimmed)?;
            trimmed
        }
        None => current.0,
    };

    let content = match request.content {
        Some(value) => {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                return Err("A beat cannot be empty.".to_string());
            }
            trimmed
        }
        None => current.1,
    };

    let characters = match request.characters {
        Some(names) => characters_to_json(Some(names))?,
        None => current.2,
    };

    connection
        .execute(
            r#"
            UPDATE story_beats
            SET beat_type = ?1, content = ?2, characters = ?3, updated_at = ?4
            WHERE id = ?5
            "#,
            params![beat_type, content, characters, timestamp(), request.beat_id],
        )
        .map_err(|error| format!("Could not update story beat: {error}"))?;

    touch_plan(&connection, &plan_id)?;
    read_plan_detail(&connection, &plan_id)
}

#[tauri::command]
pub fn storyplan_beat_delete(request: StoryBeatDeleteRequest) -> CommandResult<StoryPlanDetail> {
    let connection = open_project_database(&request.project_path)?;
    let scene_id = scene_id_for_beat(&connection, &request.beat_id)?;
    let plan_id = plan_id_for_scene(&connection, &scene_id)?;

    connection
        .execute(
            "DELETE FROM story_beats WHERE id = ?1",
            params![request.beat_id],
        )
        .map_err(|error| format!("Could not delete story beat: {error}"))?;

    delete_candidates_for(&connection, std::slice::from_ref(&request.beat_id))?;
    touch_plan(&connection, &plan_id)?;
    read_plan_detail(&connection, &plan_id)
}

#[tauri::command]
pub fn storyplan_beat_lock(request: StoryBeatLockRequest) -> CommandResult<StoryPlanDetail> {
    let connection = open_project_database(&request.project_path)?;
    let scene_id = scene_id_for_beat(&connection, &request.beat_id)?;
    let plan_id = plan_id_for_scene(&connection, &scene_id)?;

    connection
        .execute(
            "UPDATE story_beats SET locked = ?1, updated_at = ?2 WHERE id = ?3",
            params![if request.locked { 1 } else { 0 }, timestamp(), request.beat_id],
        )
        .map_err(|error| format!("Could not update beat lock: {error}"))?;

    touch_plan(&connection, &plan_id)?;
    read_plan_detail(&connection, &plan_id)
}

// ── Reorder (scenes and beats) ──

fn swap_sort_order(
    connection: &Connection,
    table: &str,
    parent_column: &str,
    id: &str,
    direction: &str,
) -> CommandResult<()> {
    let (current_order, parent_id): (i64, String) = connection
        .query_row(
            &format!("SELECT sort_order, {parent_column} FROM {table} WHERE id = ?1"),
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| format!("Could not find that {table} row."))?;

    let neighbour = if direction == "up" {
        connection.query_row(
            &format!(
                "SELECT id, sort_order FROM {table} WHERE {parent_column} = ?1 AND sort_order < ?2 ORDER BY sort_order DESC LIMIT 1"
            ),
            params![parent_id, current_order],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
    } else {
        connection.query_row(
            &format!(
                "SELECT id, sort_order FROM {table} WHERE {parent_column} = ?1 AND sort_order > ?2 ORDER BY sort_order ASC LIMIT 1"
            ),
            params![parent_id, current_order],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
    };

    let Ok((neighbour_id, neighbour_order)) = neighbour else {
        return Ok(()); // Already at the edge — nothing to swap.
    };

    connection
        .execute(
            &format!("UPDATE {table} SET sort_order = ?1, updated_at = ?3 WHERE id = ?2"),
            params![neighbour_order, id, timestamp()],
        )
        .map_err(|error| format!("Could not reorder: {error}"))?;
    connection
        .execute(
            &format!("UPDATE {table} SET sort_order = ?1, updated_at = ?3 WHERE id = ?2"),
            params![current_order, neighbour_id, timestamp()],
        )
        .map_err(|error| format!("Could not reorder: {error}"))?;

    Ok(())
}

#[tauri::command]
pub fn storyplan_reorder(request: StoryReorderRequest) -> CommandResult<StoryPlanDetail> {
    let connection = open_project_database(&request.project_path)?;
    let direction = request.direction.trim().to_lowercase();
    if direction != "up" && direction != "down" {
        return Err("Reorder direction must be up or down.".to_string());
    }

    let plan_id = match request.kind.trim().to_lowercase().as_str() {
        "scene" => {
            let plan_id = plan_id_for_scene(&connection, &request.id)?;
            swap_sort_order(&connection, "story_scenes", "plan_id", &request.id, &direction)?;
            plan_id
        }
        "beat" => {
            let scene_id = scene_id_for_beat(&connection, &request.id)?;
            let plan_id = plan_id_for_scene(&connection, &scene_id)?;
            swap_sort_order(&connection, "story_beats", "scene_id", &request.id, &direction)?;
            plan_id
        }
        other => {
            return Err(format!("Reorder kind must be scene or beat, got: {other}"));
        }
    };

    touch_plan(&connection, &plan_id)?;
    read_plan_detail(&connection, &plan_id)
}

// ── Candidate commands ──

#[tauri::command]
pub fn storyplan_candidate_store(request: StoryCandidateStoreRequest) -> CommandResult<StoryCandidate> {
    let connection = open_project_database(&request.project_path)?;
    let target_kind = request.target_kind.trim().to_lowercase();
    if !["plan", "scene", "beat", "script"].contains(&target_kind.as_str()) {
        return Err("Candidate target must be plan, scene, beat, or script.".to_string());
    }
    let content = request.content.trim().to_string();
    if content.is_empty() {
        return Err("Candidate content cannot be empty.".to_string());
    }

    let candidate_id = format!("candidate_{}", timestamp_nanos());
    let now = timestamp();
    connection
        .execute(
            r#"
            INSERT INTO story_candidates (id, target_kind, target_id, provider, model, prompt_summary, candidate_index, content, status, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending', ?9)
            "#,
            params![
                candidate_id,
                target_kind,
                request.target_id,
                request.provider,
                request.model,
                request.prompt_summary,
                request.candidate_index,
                content,
                now
            ],
        )
        .map_err(|error| format!("Could not store story candidate: {error}"))?;

    connection
        .query_row(
            r#"
            SELECT id, target_kind, target_id, provider, model, prompt_summary, candidate_index, content, status, created_at
            FROM story_candidates WHERE id = ?1
            "#,
            params![candidate_id],
            |row| {
                Ok(StoryCandidate {
                    id: row.get(0)?,
                    target_kind: row.get(1)?,
                    target_id: row.get(2)?,
                    provider: row.get(3)?,
                    model: row.get(4)?,
                    prompt_summary: row.get(5)?,
                    candidate_index: row.get(6)?,
                    content: row.get(7)?,
                    status: row.get(8)?,
                    created_at: row.get(9)?,
                })
            },
        )
        .map_err(|error| format!("Could not read stored candidate: {error}"))
}

#[tauri::command]
pub fn storyplan_candidate_list(
    project_path: String,
    target_kind: String,
    target_id: String,
) -> CommandResult<Vec<StoryCandidate>> {
    let connection = open_project_database(&project_path)?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT id, target_kind, target_id, provider, model, prompt_summary, candidate_index, content, status, created_at
            FROM story_candidates
            WHERE target_kind = ?1 AND target_id = ?2
            ORDER BY candidate_index ASC, created_at DESC
            "#,
        )
        .map_err(|error| format!("Could not read story candidates: {error}"))?;

    let rows = statement
        .query_map(params![target_kind, target_id], |row| {
            Ok(StoryCandidate {
                id: row.get(0)?,
                target_kind: row.get(1)?,
                target_id: row.get(2)?,
                provider: row.get(3)?,
                model: row.get(4)?,
                prompt_summary: row.get(5)?,
                candidate_index: row.get(6)?,
                content: row.get(7)?,
                status: row.get(8)?,
                created_at: row.get(9)?,
            })
        })
        .map_err(|error| format!("Could not read story candidates: {error}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not read story candidates: {error}"))
}

#[tauri::command]
pub fn storyplan_candidate_resolve(request: StoryCandidateResolveRequest) -> CommandResult<StoryCandidate> {
    let connection = open_project_database(&request.project_path)?;
    let resolution = request.resolution.trim().to_lowercase();
    if resolution != "accepted" && resolution != "rejected" {
        return Err("Resolution must be accepted or rejected.".to_string());
    }

    let (target_kind, target_id): (String, String) = connection
        .query_row(
            "SELECT target_kind, target_id FROM story_candidates WHERE id = ?1",
            params![request.candidate_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| "Could not find that story candidate.".to_string())?;

    connection
        .execute(
            "UPDATE story_candidates SET status = ?1 WHERE id = ?2",
            params![resolution, request.candidate_id],
        )
        .map_err(|error| format!("Could not resolve story candidate: {error}"))?;

    // One accepted candidate per target: siblings still pending are rejected.
    if resolution == "accepted" {
        connection
            .execute(
                r#"
                UPDATE story_candidates
                SET status = 'rejected'
                WHERE target_kind = ?1 AND target_id = ?2 AND id != ?3 AND status = 'pending'
                "#,
                params![target_kind, target_id, request.candidate_id],
            )
            .map_err(|error| format!("Could not resolve sibling candidates: {error}"))?;
    }

    connection
        .query_row(
            r#"
            SELECT id, target_kind, target_id, provider, model, prompt_summary, candidate_index, content, status, created_at
            FROM story_candidates WHERE id = ?1
            "#,
            params![request.candidate_id],
            |row| {
                Ok(StoryCandidate {
                    id: row.get(0)?,
                    target_kind: row.get(1)?,
                    target_id: row.get(2)?,
                    provider: row.get(3)?,
                    model: row.get(4)?,
                    prompt_summary: row.get(5)?,
                    candidate_index: row.get(6)?,
                    content: row.get(7)?,
                    status: row.get(8)?,
                    created_at: row.get(9)?,
                })
            },
        )
        .map_err(|error| format!("Could not read resolved candidate: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::schema::{run_migrations, STORY_PLAN_SCHEMA};

    fn test_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        run_migrations(&mut conn).unwrap();
        conn
    }

    fn insert_plan(conn: &Connection, id: &str) {
        conn.execute(
            "INSERT INTO story_plans (id, project_name, status, created_at, updated_at) VALUES (?1, 'Test Novel', 'draft', '1', '1')",
            params![id],
        )
        .unwrap();
    }

    fn insert_scene(conn: &Connection, id: &str, plan_id: &str) {
        conn.execute(
            "INSERT INTO story_scenes (id, plan_id, title, sort_order, created_at, updated_at) VALUES (?1, ?2, 'Scene', 0, '1', '1')",
            params![id, plan_id],
        )
        .unwrap();
    }

    fn insert_beat(conn: &Connection, id: &str, scene_id: &str, locked: i64) {
        conn.execute(
            "INSERT INTO story_beats (id, scene_id, beat_type, content, locked, sort_order, created_at, updated_at) VALUES (?1, ?2, 'action', 'Something happens', ?3, 0, '1', '1')",
            params![id, scene_id, locked],
        )
        .unwrap();
    }

    fn count(conn: &Connection, query: &str) -> i64 {
        conn.query_row(query, [], |row| row.get(0)).unwrap()
    }

    #[test]
    fn story_plan_schema_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(STORY_PLAN_SCHEMA).unwrap();
        // Re-applying must not error (IF NOT EXISTS everywhere).
        conn.execute_batch(STORY_PLAN_SCHEMA).unwrap();
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name LIKE 'story_%'"),
            4
        );
    }

    #[test]
    fn migration_creates_story_plan_tables() {
        let conn = test_db();
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(tables.contains(&"story_plans".to_string()));
        assert!(tables.contains(&"story_scenes".to_string()));
        assert!(tables.contains(&"story_beats".to_string()));
        assert!(tables.contains(&"story_candidates".to_string()));
    }

    #[test]
    fn migration_records_story_plan_version() {
        let conn = test_db();
        let version: i64 = conn
            .query_row(
                "SELECT version FROM schema_migrations WHERE name = 'story_plan'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, 3);
    }

    #[test]
    fn delete_plan_cascades_scenes_and_beats() {
        let conn = test_db();
        insert_plan(&conn, "plan_1");
        insert_scene(&conn, "scene_1", "plan_1");
        insert_beat(&conn, "beat_1", "scene_1", 0);

        conn.execute("DELETE FROM story_plans WHERE id = 'plan_1'", []).unwrap();
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM story_scenes"), 0);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM story_beats"), 0);
    }

    #[test]
    fn delete_scene_cascades_beats_but_keeps_plan() {
        let conn = test_db();
        insert_plan(&conn, "plan_1");
        insert_scene(&conn, "scene_1", "plan_1");
        insert_beat(&conn, "beat_1", "scene_1", 0);

        conn.execute("DELETE FROM story_scenes WHERE id = 'scene_1'", []).unwrap();
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM story_plans"), 1);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM story_beats"), 0);
    }

    #[test]
    fn beat_lock_persists_through_update() {
        let conn = test_db();
        insert_plan(&conn, "plan_1");
        insert_scene(&conn, "scene_1", "plan_1");
        insert_beat(&conn, "beat_1", "scene_1", 0);

        conn.execute("UPDATE story_beats SET locked = 1, content = 'New content' WHERE id = 'beat_1'", []).unwrap();

        let beats = read_beats(&conn, "scene_1").unwrap();
        assert_eq!(beats.len(), 1);
        assert!(beats[0].locked, "Lock flag must survive content updates");
        assert_eq!(beats[0].content, "New content");
    }

    #[test]
    fn characters_round_trip_as_string_array() {
        let conn = test_db();
        insert_plan(&conn, "plan_1");
        insert_scene(&conn, "scene_1", "plan_1");
        insert_beat(&conn, "beat_1", "scene_1", 0);

        let json = characters_to_json(Some(vec!["  Mara ".to_string(), "Jonah".to_string(), " ".to_string()])).unwrap();
        conn.execute("UPDATE story_beats SET characters = ?1 WHERE id = 'beat_1'", params![json]).unwrap();

        let beats = read_beats(&conn, "scene_1").unwrap();
        assert_eq!(beats[0].characters, Some(vec!["Mara".to_string(), "Jonah".to_string()]));
    }

    #[test]
    fn candidates_lifecycle_pending_accept_rejects_siblings() {
        let conn = test_db();
        insert_plan(&conn, "plan_1");
        insert_scene(&conn, "scene_1", "plan_1");

        for (index, id) in ["candidate_1", "candidate_2", "candidate_3"].iter().enumerate() {
            conn.execute(
                "INSERT INTO story_candidates (id, target_kind, target_id, provider, model, candidate_index, content, status, created_at) VALUES (?1, 'scene', 'scene_1', 'anthropic', 'claude-sonnet-4-5', ?2, 'variant text', 'pending', '1')",
                params![id, index as i64],
            )
            .unwrap();
        }

        // Accept candidate_2.
        conn.execute("UPDATE story_candidates SET status = 'accepted' WHERE id = 'candidate_2'", []).unwrap();
        conn.execute(
            "UPDATE story_candidates SET status = 'rejected' WHERE target_kind = 'scene' AND target_id = 'scene_1' AND id != 'candidate_2' AND status = 'pending'",
            [],
        )
        .unwrap();

        assert_eq!(count(&conn, "SELECT COUNT(*) FROM story_candidates WHERE status = 'pending'"), 0);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM story_candidates WHERE status = 'accepted'"), 1);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM story_candidates WHERE status = 'rejected'"), 2);
    }

    #[test]
    fn delete_candidates_for_cleans_polymorphic_targets() {
        let conn = test_db();
        insert_plan(&conn, "plan_1");
        conn.execute(
            "INSERT INTO story_candidates (id, target_kind, target_id, provider, model, candidate_index, content, status, created_at) VALUES ('c1', 'plan', 'plan_1', 'ollama', 'llama3.2', 0, 'text', 'pending', '1')",
            [],
        )
        .unwrap();

        delete_candidates_for(&conn, &["plan_1".to_string()]).unwrap();
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM story_candidates"), 0);
    }

    #[test]
    fn read_plan_detail_nests_scenes_and_beats() {
        let conn = test_db();
        insert_plan(&conn, "plan_1");
        insert_scene(&conn, "scene_1", "plan_1");
        insert_scene(&conn, "scene_2", "plan_1");
        insert_beat(&conn, "beat_1", "scene_1", 0);
        insert_beat(&conn, "beat_2", "scene_1", 1);

        let detail = read_plan_detail(&conn, "plan_1").unwrap();
        assert_eq!(detail.plan.id, "plan_1");
        assert_eq!(detail.scenes.len(), 2);
        assert_eq!(detail.scenes[0].beats.len(), 2);
        assert_eq!(detail.scenes[1].beats.len(), 0);
        assert!(detail.scenes[0].beats[1].locked);
    }

    #[test]
    fn read_plan_detail_errors_on_missing_plan() {
        let conn = test_db();
        let result = read_plan_detail(&conn, "plan_missing");
        assert!(result.is_err());
    }

    #[test]
    fn validate_beat_type_rejects_unknown() {
        assert!(validate_beat_type("action").is_ok());
        assert!(validate_beat_type("montage").is_err());
    }
}
