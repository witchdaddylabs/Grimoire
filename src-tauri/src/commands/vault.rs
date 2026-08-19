use super::*;
use crate::db::{
    clear_item_chunks, ensure_hierarchy_node, ensure_import_drawer, import_progress_labels,
    item_path, item_type, next_sort_order, read_item_detail, sync_item_chunks, MAX_IMPORT_WORDS,
};
use crate::helpers::{count_words, normalize_text, timestamp, timestamp_nanos};

#[tauri::command]
pub fn db_get_item(project_path: String, item_id: String) -> CommandResult<VaultItemDetail> {
    let connection = open_project_database(&project_path)?;
    read_item_detail(&connection, &item_id)
}

#[tauri::command]
pub fn db_update_item(request: ItemUpdateRequest) -> CommandResult<VaultItemDetail> {
    let connection = open_project_database(&request.project_path)?;
    let title = request.title.trim();
    if title.is_empty() {
        return Err("The Canvas title cannot be empty.".to_string());
    }

    let content = normalize_text(&request.content);
    let word_count = count_words(&content);
    let updated_at = timestamp();
    let current_item_type = item_type(&connection, &request.item_id)?;

    connection
        .execute(
            r#"
            UPDATE items
            SET title = ?1,
                content = ?2,
                plain_text = ?2,
                word_count = ?3,
                updated_at = ?4
            WHERE id = ?5
              AND archived_at IS NULL
            "#,
            params![title, content, word_count, updated_at, request.item_id],
        )
        .map_err(|error| format!("Could not update Canvas item: {error}"))?;

    if connection.changes() == 0 {
        return Err("Could not find that Vault item.".to_string());
    }

    let path = item_path(&connection, &request.item_id, title)?;
    let chunk_count = sync_item_chunks(
        &connection,
        &request.item_id,
        title,
        &current_item_type,
        &path,
        &content,
    )?;

    if chunk_count == 0 {
        clear_item_chunks(&connection, &request.item_id)?;
    }

    read_item_detail(&connection, &request.item_id)
}

#[tauri::command]
pub fn db_import_text(request: ImportTextRequest) -> CommandResult<ImportTextResponse> {
    let connection = open_project_database(&request.project_path)?;
    let content = normalize_text(&request.content);
    if content.trim().is_empty() {
        return Err("Import text is empty.".to_string());
    }
    let word_count = count_words(&content);
    if word_count > MAX_IMPORT_WORDS as usize {
        return Err(format!(
            "Import is limited to {MAX_IMPORT_WORDS} words per item. Split this file and import the next section separately."
        ));
    }

    let title = if request.title.trim().is_empty() {
        request
            .source_name
            .as_deref()
            .unwrap_or("Imported writing")
            .trim()
            .to_string()
    } else {
        request.title.trim().to_string()
    };

    let drawer_id = ensure_import_drawer(&connection)?;
    let item_id = format!("item_import_{}", timestamp_nanos());
    let now = timestamp();
    let sort_order = next_sort_order(&connection, "items", "drawer_id", &drawer_id)?;

    connection
        .execute(
            r#"
            INSERT INTO items (
              id, drawer_id, title, item_type, content, plain_text, word_count,
              source_kind, source_path, sort_order, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, 'note', ?4, ?4, ?5, 'import', ?6, ?7, ?8, ?8)
            "#,
            params![
                item_id,
                drawer_id,
                title,
                content,
                word_count,
                request.source_name,
                sort_order,
                now
            ],
        )
        .map_err(|error| format!("Could not create imported Vault item: {error}"))?;

    let path = item_path(&connection, &item_id, &title)?;
    let created_chunks = sync_item_chunks(&connection, &item_id, &title, "note", &path, &content)?;
    let item = read_item_detail(&connection, &item_id)?;

    Ok(ImportTextResponse {
        item,
        progress_labels: import_progress_labels(),
        created_chunks,
    })
}

#[tauri::command]
pub fn db_archive_item(request: ItemArchiveRequest) -> CommandResult<VaultTreeResponse> {
    let connection = open_project_database(&request.project_path)?;
    clear_item_chunks(&connection, &request.item_id)?;

    connection
        .execute(
            "UPDATE items SET archived_at = ?1, updated_at = ?1 WHERE id = ?2 AND archived_at IS NULL",
            params![timestamp(), request.item_id],
        )
        .map_err(|error| format!("Could not archive Vault item: {error}"))?;

    if connection.changes() == 0 {
        return Err("Could not find that active Vault item to archive.".to_string());
    }

    crate::db::read_vault_tree(&connection)
}

#[tauri::command]
pub fn db_delete_item(request: ItemDeleteRequest) -> CommandResult<VaultTreeResponse> {
    let connection = open_project_database(&request.project_path)?;
    clear_item_chunks(&connection, &request.item_id)?;

    connection
        .execute("DELETE FROM items WHERE id = ?1", params![request.item_id])
        .map_err(|error| format!("Could not delete Vault item: {error}"))?;

    if connection.changes() == 0 {
        return Err("Could not find that Vault item to delete.".to_string());
    }

    crate::db::read_vault_tree(&connection)
}

#[tauri::command]
pub fn db_create_vault_node(
    request: CreateVaultNodeRequest,
) -> CommandResult<CreateVaultNodeResponse> {
    let connection = open_project_database(&request.project_path)?;
    let now = timestamp();
    let node_type = request.node_type.trim().to_lowercase();
    let node_id = format!("{}_{}", node_type, timestamp_nanos());
    let name = request.name.trim().to_string();

    if name.is_empty() {
        return Err("Name is required.".to_string());
    }

    match node_type.as_str() {
        "wing" => {
            let sort_order: i64 = connection
                .query_row(
                    "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM wings",
                    [],
                    |row| row.get(0),
                )
                .map_err(|error| format!("Could not calculate wing sort order: {error}"))?;
            connection
                .execute(
                    r#"
                    INSERT INTO wings (id, name, description, sort_order, created_at, updated_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?5)
                    "#,
                    params![
                        node_id,
                        name,
                        request.description.unwrap_or_default(),
                        sort_order,
                        now
                    ],
                )
                .map_err(|error| format!("Could not create Vault wing: {error}"))?;
        }
        "hall" => {
            let parent_id = request.parent_id.ok_or("Wing ID is required for halls.")?;
            ensure_hierarchy_node(&connection, "wings", &parent_id, "wing")?;
            let sort_order = next_sort_order(&connection, "halls", "wing_id", &parent_id)?;
            connection
                .execute(
                    r#"
                    INSERT INTO halls (id, wing_id, name, description, sort_order, created_at, updated_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
                    "#,
                    params![node_id, parent_id, name, request.description.unwrap_or_default(), sort_order, now],
                )
                .map_err(|error| format!("Could not create Vault hall: {error}"))?;
        }
        "room" => {
            let parent_id = request.parent_id.ok_or("Hall ID is required for rooms.")?;
            ensure_hierarchy_node(&connection, "halls", &parent_id, "hall")?;
            let sort_order = next_sort_order(&connection, "rooms", "hall_id", &parent_id)?;
            connection
                .execute(
                    r#"
                    INSERT INTO rooms (id, hall_id, name, description, sort_order, created_at, updated_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
                    "#,
                    params![node_id, parent_id, name, request.description.unwrap_or_default(), sort_order, now],
                )
                .map_err(|error| format!("Could not create Vault room: {error}"))?;
        }
        "drawer" => {
            let parent_id = request
                .parent_id
                .ok_or("Room ID is required for drawers.")?;
            ensure_hierarchy_node(&connection, "rooms", &parent_id, "room")?;
            let sort_order = next_sort_order(&connection, "drawers", "room_id", &parent_id)?;
            connection
                .execute(
                    r#"
                    INSERT INTO drawers (id, room_id, name, description, sort_order, created_at, updated_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
                    "#,
                    params![node_id, parent_id, name, request.description.unwrap_or_default(), sort_order, now],
                )
                .map_err(|error| format!("Could not create Vault drawer: {error}"))?;
        }
        "item" => {
            let parent_id = request
                .parent_id
                .ok_or("Drawer ID is required for items.")?;
            ensure_hierarchy_node(&connection, "drawers", &parent_id, "drawer")?;
            let item_type = request
                .item_type
                .as_deref()
                .unwrap_or("note")
                .trim()
                .to_lowercase();
            let valid_types = [
                "chapter",
                "scene",
                "character",
                "location",
                "lore",
                "timeline",
                "faction",
                "research",
                "note",
            ];
            if !valid_types.contains(&item_type.as_str()) {
                return Err(format!(
                    "Invalid item type. Choose one of: {}",
                    valid_types.join(", ")
                ));
            }
            let sort_order = next_sort_order(&connection, "items", "drawer_id", &parent_id)?;
            connection
                .execute(
                    r#"
                    INSERT INTO items (
                        id, drawer_id, title, item_type, content, plain_text, word_count,
                        source_kind, sort_order, created_at, updated_at
                    )
                    VALUES (?1, ?2, ?3, ?4, '', '', 0, 'manual', ?5, ?6, ?6)
                    "#,
                    params![node_id, parent_id, name, item_type, sort_order, now],
                )
                .map_err(|error| format!("Could not create Vault item: {error}"))?;
        }
        _ => {
            return Err("Node type must be wing, hall, room, drawer, or item.".to_string());
        }
    }

    Ok(CreateVaultNodeResponse {
        id: node_id,
        node_type,
        tree: crate::db::read_vault_tree(&connection)?,
    })
}
