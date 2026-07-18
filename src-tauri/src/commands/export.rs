use super::*;
use crate::db::{item_path, read_banned_words, read_item_detail, read_vault_tree};
use crate::helpers::timestamp;
use serde_json::json;

#[tauri::command]
pub fn export_item_markdown(request: ExportItemRequest) -> CommandResult<ExportResponse> {
    let project_dir = super::validate_project_dir(PathBuf::from(&request.project_path))?;
    let connection = open_project_database(&request.project_path)?;
    let item = read_item_detail(&connection, &request.item_id)?;
    let export_dir = project_dir.join("exports");
    fs::create_dir_all(&export_dir)
        .map_err(|error| format!("Could not create export folder: {error}"))?;
    let file_path = export_dir.join(format!("{}.md", sanitize_filename(&item.title)));
    let markdown = format!("# {}\n\n{}\n", item.title, item.content.trim());
    fs::write(&file_path, markdown)
        .map_err(|error| format!("Could not write Markdown export: {error}"))?;

    Ok(ExportResponse {
        path: file_path.to_string_lossy().to_string(),
        message: "Markdown export written.".to_string(),
    })
}

#[tauri::command]
pub fn export_project_json(project_path: String) -> CommandResult<ExportResponse> {
    let project_dir = super::validate_project_dir(PathBuf::from(&project_path))?;
    let metadata = super::read_metadata(&project_dir)?;
    let connection = open_project_database(&project_path)?;
    let tree = read_vault_tree(&connection)?;
    let wards = read_banned_words(&connection)?;
    let export_dir = project_dir.join("exports");
    fs::create_dir_all(&export_dir)
        .map_err(|error| format!("Could not create export folder: {error}"))?;
    let file_path = export_dir.join(format!("grimoire-export-{}.json", timestamp()));
    let payload = json!({
        "exportedAt": timestamp(),
        "project": {
            "name": metadata.name,
            "schemaVersion": metadata.schema_version,
            "projectPath": metadata.project_path
        },
        "vault": tree,
        "wards": wards
    });
    let raw = serde_json::to_string_pretty(&payload)
        .map_err(|error| format!("Could not serialize project export: {error}"))?;
    fs::write(&file_path, raw)
        .map_err(|error| format!("Could not write project export: {error}"))?;

    Ok(ExportResponse {
        path: file_path.to_string_lossy().to_string(),
        message: "Project JSON export written without secrets or hidden prompts.".to_string(),
    })
}

#[tauri::command]
pub fn export_vault_items_json(project_path: String) -> CommandResult<ExportResponse> {
    let project_dir = super::validate_project_dir(PathBuf::from(&project_path))?;
    let metadata = super::read_metadata(&project_dir)?;
    let connection = open_project_database(&project_path)?;
    let export_dir = project_dir.join("exports");
    fs::create_dir_all(&export_dir)
        .map_err(|error| format!("Could not create export folder: {error}"))?;
    let file_path = export_dir.join(format!("grimoire-vault-items-{}.json", timestamp()));

    let mut statement = connection
        .prepare(
            r#"
            SELECT
              id,
              title,
              item_type,
              COALESCE(content, ''),
              COALESCE(plain_text, ''),
              word_count,
              source_kind,
              source_path,
              created_at,
              updated_at
            FROM items
            WHERE archived_at IS NULL
            ORDER BY sort_order, updated_at DESC
            "#,
        )
        .map_err(|error| format!("Could not prepare Vault items export: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let path = item_path(&connection, &id, &title).unwrap_or_else(|_| title.clone());
            Ok(json!({
                "id": id,
                "title": title,
                "itemType": row.get::<_, String>(2)?,
                "content": row.get::<_, String>(3)?,
                "plainText": row.get::<_, String>(4)?,
                "wordCount": row.get::<_, i64>(5)?,
                "path": path,
                "sourceKind": row.get::<_, String>(6)?,
                "sourcePath": row.get::<_, Option<String>>(7)?,
                "createdAt": row.get::<_, String>(8)?,
                "updatedAt": row.get::<_, String>(9)?,
            }))
        })
        .map_err(|error| format!("Could not query Vault items for export: {error}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|error| format!("Could not read Vault export row: {error}"))?);
    }

    let payload = json!({
        "exportedAt": timestamp(),
        "project": {
            "name": metadata.name,
            "schemaVersion": metadata.schema_version,
            "projectPath": metadata.project_path
        },
        "items": items
    });
    let raw = serde_json::to_string_pretty(&payload)
        .map_err(|error| format!("Could not serialize Vault items export: {error}"))?;
    fs::write(&file_path, raw)
        .map_err(|error| format!("Could not write Vault items export: {error}"))?;

    Ok(ExportResponse {
        path: file_path.to_string_lossy().to_string(),
        message: "Vault items JSON export written.".to_string(),
    })
}

#[tauri::command]
pub fn manuscript_export(request: ManuscriptExportRequest) -> CommandResult<ExportResponse> {
    let project_dir = super::validate_project_dir(PathBuf::from(&request.project_path))?;
    let connection = open_project_database(&request.project_path)?;
    let export_dir = project_dir.join("exports");
    fs::create_dir_all(&export_dir)
        .map_err(|error| format!("Could not create export folder: {error}"))?;

    let tree = read_vault_tree(&connection)?;
    let mut markdown = String::new();
    markdown.push_str(&format!("# {}\n\n", request.project_name));

    for wing in &tree.wings {
        markdown.push_str(&format!("# {}\n\n", wing.name));
        for hall in &wing.halls {
            markdown.push_str(&format!("## {}\n\n", hall.name));
            for room in &hall.rooms {
                markdown.push_str(&format!("### {}\n\n", room.name));
                for drawer in &room.drawers {
                    markdown.push_str(&format!("#### {}\n\n", drawer.name));
                    for item in &drawer.items {
                        markdown.push_str(&format!("##### {}\n\n", item.title));
                        let content = item.content.as_deref().unwrap_or("").trim();
                        if !content.is_empty() {
                            markdown.push_str(&format!("{}\n\n", content));
                        }
                    }
                }
            }
        }
    }

    let ext = if request.format.as_deref() == Some("markdown") {
        "md"
    } else {
        "md"
    };
    let file_path = export_dir.join(format!(
        "grimoire-manuscript-{}.{}",
        sanitize_filename(&request.project_name),
        ext
    ));
    fs::write(&file_path, markdown)
        .map_err(|error| format!("Could not write manuscript export: {error}"))?;

    Ok(ExportResponse {
        path: file_path.to_string_lossy().to_string(),
        message: "Manuscript export written.".to_string(),
    })
}

#[tauri::command]
pub fn reorder_item(request: ItemReorderRequest) -> CommandResult<VaultTreeResponse> {
    let connection = open_project_database(&request.project_path)?;
    let direction = request.direction.as_deref().unwrap_or("down");
    let current: i64 = connection
        .query_row(
            "SELECT sort_order FROM items WHERE id = ?1 AND archived_at IS NULL",
            params![request.item_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("Could not read item: {error}"))?;

    let drawer_id: String = connection
        .query_row(
            "SELECT drawer_id FROM items WHERE id = ?1",
            params![request.item_id],
            |row| row.get(0),
        )
        .map_err(|_| "Could not find item drawer.".to_string())?;

    let swap_with = if direction == "up" {
        connection.query_row(
            "SELECT id, sort_order FROM items WHERE drawer_id = ?1 AND sort_order < ?2 AND archived_at IS NULL ORDER BY sort_order DESC LIMIT 1",
            params![drawer_id, current],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        ).ok()
    } else if direction == "down" {
        connection.query_row(
            "SELECT id, sort_order FROM items WHERE drawer_id = ?1 AND sort_order > ?2 AND archived_at IS NULL ORDER BY sort_order ASC LIMIT 1",
            params![drawer_id, current],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        ).ok()
    } else {
        None
    };

    if let Some((other_id, other_order)) = swap_with {
        connection
            .execute(
                "UPDATE items SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
                params![other_order, timestamp(), request.item_id],
            )
            .map_err(|error| format!("Could not reorder item: {error}"))?;
        connection
            .execute(
                "UPDATE items SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
                params![current, timestamp(), other_id],
            )
            .map_err(|error| format!("Could not reorder item: {error}"))?;
    }

    read_vault_tree(&connection)
}

fn sanitize_filename(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else if character.is_whitespace() {
                '-'
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches(|character| character == '-' || character == '_');
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_filename_keeps_safe_name() {
        assert_eq!(
            sanitize_filename("Chapter 01: Bell/Bones"),
            "Chapter-01_-Bell_Bones"
        );
        assert_eq!(sanitize_filename("///"), "untitled");
    }
}
