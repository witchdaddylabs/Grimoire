#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod db;
mod errors;
mod external_vault;
mod llm;

use ai::{
    cloud_provider, AiApiKeyRequest, AiChatRequest, AiChatResponse, AiModelInfo, AiProviderKind,
    AiProviderModelsResponse, AiProviderSelectionRequest, AiProviderSettings,
    AiProviderSettingsResponse, AiProviderSettingsSaveRequest, CloudDisclosureAcceptRequest,
    CLOUD_DISCLOSURE_COPY, PROVIDERS,
};
use external_vault::parse_external_vault;
use rusqlite::{params, Connection};
use keyring::{Entry, Error as KeyringError};
use serde_json::{json, Value};
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const DATABASE_FILE: &str = "grimoire.sqlite";
const METADATA_FILE: &str = "metadata.json";
const SCHEMA_VERSION: i64 = 2;
const MAX_IMPORT_WORDS: i64 = 10_000;

type CommandResult<T> = Result<T, String>;

// ── Modules ──

mod models;
mod vault;
mod wards;

use models::*;
use vault::*;
use wards::*;

#[tauri::command]
fn app_ping() -> &'static str {
    "Grimoire desktop scaffold awake"
}

#[tauri::command]
fn project_create(request: ProjectCreateRequest) -> CommandResult<ProjectMetadata> {
    let parent_dir = match request.parent_dir {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path),
        _ => default_projects_dir()?,
    };

    let project_dir = parent_dir.join(project_folder_name(&request.name));
    fs::create_dir_all(&project_dir).map_err(|error| {
        format!(
            "Could not create project folder at {}: {error}",
            project_dir.display()
        )
    })?;

    let metadata = load_or_create_metadata(&project_dir, &request.name)?;
    initialise_database(&metadata, request.seed_demo_data.unwrap_or(false))?;
    Ok(metadata)
}

#[tauri::command]
fn project_open(project_path: String) -> CommandResult<ProjectMetadata> {
    let project_dir = validate_project_dir(PathBuf::from(project_path))?;
    let metadata = read_metadata(&project_dir)?;
    initialise_database(&metadata, false)?;
    Ok(metadata)
}

#[tauri::command]
fn project_get_metadata(project_path: String) -> CommandResult<ProjectMetadata> {
    let project_dir = validate_project_dir(PathBuf::from(project_path))?;
    read_metadata(&project_dir)
}

#[tauri::command]
fn db_init(project_path: String) -> CommandResult<ProjectMetadata> {
    let project_dir = validate_project_dir(PathBuf::from(project_path))?;
    let metadata = read_metadata(&project_dir)?;
    initialise_database(&metadata, false)?;
    Ok(metadata)
}

#[tauri::command]
fn db_get_vault_tree(project_path: String) -> CommandResult<VaultTreeResponse> {
    let project_dir = validate_project_dir(PathBuf::from(project_path))?;
    let metadata = read_metadata(&project_dir)?;
    initialise_database(&metadata, false)?;

    let connection = Connection::open(&metadata.database_path)
        .map_err(|error| format!("Could not open SQLite database: {error}"))?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| format!("Could not enable SQLite foreign keys: {error}"))?;

    read_vault_tree(&connection)
}

#[tauri::command]
fn db_get_item(project_path: String, item_id: String) -> CommandResult<VaultItemDetail> {
    let connection = open_project_database(&project_path)?;
    read_item_detail(&connection, &item_id)
}

#[tauri::command]
fn db_update_item(request: ItemUpdateRequest) -> CommandResult<VaultItemDetail> {
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
fn db_import_text(request: ImportTextRequest) -> CommandResult<ImportTextResponse> {
    let connection = open_project_database(&request.project_path)?;
    let content = normalize_text(&request.content);
    if content.trim().is_empty() {
        return Err("Import text is empty.".to_string());
    }
    let word_count = count_words(&content);
    if word_count > MAX_IMPORT_WORDS {
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
fn db_archive_item(request: ItemArchiveRequest) -> CommandResult<VaultTreeResponse> {
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

    read_vault_tree(&connection)
}

#[tauri::command]
fn db_delete_item(request: ItemDeleteRequest) -> CommandResult<VaultTreeResponse> {
    let connection = open_project_database(&request.project_path)?;
    clear_item_chunks(&connection, &request.item_id)?;

    connection
        .execute("DELETE FROM items WHERE id = ?1", params![request.item_id])
        .map_err(|error| format!("Could not delete Vault item: {error}"))?;

    if connection.changes() == 0 {
        return Err("Could not find that Vault item to delete.".to_string());
    }

    read_vault_tree(&connection)
}

#[tauri::command]
fn db_create_vault_node(request: CreateVaultNodeRequest) -> CommandResult<CreateVaultNodeResponse> {
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
        tree: read_vault_tree(&connection)?,
    })
}

#[tauri::command]
fn db_search_chunks(request: SearchChunksRequest) -> CommandResult<SearchChunksResponse> {
    let connection = open_project_database(&request.project_path)?;
    let query = request.query.trim().to_string();
    let fts_query = if request.mode.as_deref() == Some("broad") {
        fts_query_broad(&query)?
    } else {
        fts_query(&query)?
    };
    let limit = request.limit.unwrap_or(8).clamp(1, 24);
    let mut statement = connection
        .prepare(
            r#"
            SELECT
              chunk_id,
              item_id,
              title,
              item_type,
              vault_path,
              text,
              bm25(item_chunks_fts) AS rank
            FROM item_chunks_fts
            WHERE item_chunks_fts MATCH ?1
            ORDER BY rank
            LIMIT ?2
            "#,
        )
        .map_err(|error| format!("Could not prepare Vault search: {error}"))?;

    let mapped = statement
        .query_map(params![fts_query, limit], |row| {
            let raw_score: f64 = row.get(6)?;
            let score = 0.0 - raw_score;
            Ok(SearchChunkResult {
                chunk_id: row.get(0)?,
                item_id: row.get(1)?,
                title: row.get(2)?,
                item_type: row.get(3)?,
                vault_path: row.get(4)?,
                snippet: row.get(5)?,
                score,
                confidence: confidence_for_score(score),
            })
        })
        .map_err(|error| format!("Could not search Vault chunks: {error}"))?;

    let mut results = Vec::new();
    for result in mapped {
        results.push(result.map_err(|error| format!("Could not read search result: {error}"))?);
    }

    Ok(SearchChunksResponse {
        query,
        confidence: aggregate_confidence(&results),
        results,
    })
}

#[tauri::command]
fn wards_list(project_path: String) -> CommandResult<Vec<BannedWord>> {
    let connection = open_project_database(&project_path)?;
    read_banned_words(&connection)
}

#[tauri::command]
fn wards_add(request: WardPhraseRequest) -> CommandResult<Vec<BannedWord>> {
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
            params![format!("ward_{}", timestamp_nanos()), value, severity, now],
        )
        .map_err(|error| format!("Could not save ward phrase: {error}"))?;

    read_banned_words(&connection)
}

#[tauri::command]
fn wards_remove(project_path: String, id: String) -> CommandResult<Vec<BannedWord>> {
    let connection = open_project_database(&project_path)?;
    connection
        .execute("DELETE FROM banned_words WHERE id = ?1", params![id])
        .map_err(|error| format!("Could not remove ward phrase: {error}"))?;
    read_banned_words(&connection)
}

#[tauri::command]
fn wards_scan(request: WardScanRequest) -> CommandResult<WardScanResponse> {
    let connection = open_project_database(&request.project_path)?;
    let words = read_banned_words(&connection)?;
    Ok(scan_wards(&words, &request.text))
}

#[tauri::command]
fn ollama_get_status(project_path: String) -> CommandResult<OllamaStatus> {
    let response = ai_list_models(project_path, AiProviderKind::Ollama)?;
    Ok(OllamaStatus {
        base_url: "http://127.0.0.1:11434".to_string(),
        reachable: response.reachable,
        models: response.models.into_iter().map(Into::into).collect(),
        selected_model: response.selected_model,
        message: response.message,
    })
}

#[tauri::command]
fn ollama_select_model(request: OllamaSelectModelRequest) -> CommandResult<OllamaStatus> {
    ai_save_provider_settings(AiProviderSettingsSaveRequest {
        project_path: request.project_path.clone(),
        provider: AiProviderKind::Ollama,
        base_url: None,
        selected_model: Some(request.model),
    })?;
    ollama_get_status(request.project_path)
}

#[tauri::command]
fn ollama_chat(request: OllamaChatRequest) -> CommandResult<OllamaChatResponse> {
    let response = ai_chat(AiChatRequest {
        project_path: request.project_path,
        provider: AiProviderKind::Ollama,
        model: request.model,
        prompt: request.prompt,
        grounded_context: request.context.unwrap_or_default(),
    })?;
    Ok(OllamaChatResponse {
        model: response.model,
        text: response.text,
    })
}

#[tauri::command]
fn ai_get_provider_settings(project_path: String) -> CommandResult<AiProviderSettingsResponse> {
    let connection = open_project_database(&project_path)?;
    let mut active_provider = get_active_provider(&connection)?;
    if !PROVIDERS.contains(&active_provider) {
        active_provider = AiProviderKind::Ollama;
        set_setting(&connection, "ai.activeProvider", active_provider.as_key())?;
    }
    let providers = PROVIDERS
        .iter()
        .copied()
        .map(|provider| provider_settings(&connection, provider))
        .collect::<CommandResult<Vec<_>>>()?;

    Ok(AiProviderSettingsResponse {
        active_provider,
        providers,
        cloud_disclosure_copy: CLOUD_DISCLOSURE_COPY.to_string(),
    })
}

#[tauri::command]
fn ai_save_provider_settings(
    request: AiProviderSettingsSaveRequest,
) -> CommandResult<AiProviderSettingsResponse> {
    let connection = open_project_database(&request.project_path)?;
    if let Some(base_url) = request.base_url.as_deref() {
        let value = base_url.trim();
        if !value.is_empty() {
            set_setting(
                &connection,
                &provider_setting_key(request.provider, "baseUrl"),
                value,
            )?;
        }
    }
    if let Some(model) = request.selected_model.as_deref() {
        let value = model.trim();
        if !value.is_empty() {
            set_setting(
                &connection,
                &provider_setting_key(request.provider, "selectedModel"),
                value,
            )?;
        }
    }
    ai_get_provider_settings(request.project_path)
}

#[tauri::command]
fn ai_set_api_key(request: AiApiKeyRequest) -> CommandResult<AiProviderSettingsResponse> {
    if !cloud_provider(&request.provider) {
        return Err("Ollama does not need an API key.".to_string());
    }
    let connection = open_project_database(&request.project_path)?;
    let api_key = request.api_key.trim();
    if api_key.is_empty() {
        return Err("API key cannot be empty.".to_string());
    }
    set_api_key_secret(&request.project_path, request.provider, api_key)?;
    set_setting(
        &connection,
        &provider_setting_key(request.provider, "apiKeyPresent"),
        "true",
    )?;
    ai_get_provider_settings(request.project_path)
}

#[tauri::command]
fn ai_delete_api_key(
    project_path: String,
    provider: AiProviderKind,
) -> CommandResult<AiProviderSettingsResponse> {
    let connection = open_project_database(&project_path)?;
    delete_api_key_secret(&project_path, provider)?;
    set_setting(
        &connection,
        &provider_setting_key(provider, "apiKeyPresent"),
        "false",
    )?;
    ai_get_provider_settings(project_path)
}

#[tauri::command]
fn ai_accept_cloud_disclosure(
    request: CloudDisclosureAcceptRequest,
) -> CommandResult<AiProviderSettingsResponse> {
    if !cloud_provider(&request.provider) {
        return ai_get_provider_settings(request.project_path);
    }
    let connection = open_project_database(&request.project_path)?;
    set_setting(
        &connection,
        &provider_setting_key(request.provider, "disclosureAcceptedAt"),
        &timestamp(),
    )?;
    ai_get_provider_settings(request.project_path)
}

#[tauri::command]
fn ai_select_provider(
    request: AiProviderSelectionRequest,
) -> CommandResult<AiProviderSettingsResponse> {
    let connection = open_project_database(&request.project_path)?;
    set_setting(&connection, "ai.activeProvider", request.provider.as_key())?;
    ai_get_provider_settings(request.project_path)
}

#[tauri::command]
fn ai_list_models(
    project_path: String,
    provider: AiProviderKind,
) -> CommandResult<AiProviderModelsResponse> {
    let connection = open_project_database(&project_path)?;
    match provider {
        AiProviderKind::Ollama => list_ollama_models(&connection),
        _ => {
            let settings = provider_settings(&connection, provider)?;
            let selected_model = settings
                .selected_model
                .or_else(|| provider.default_model().map(ToString::to_string));
            let models = selected_model
                .iter()
                .map(|name| AiModelInfo {
                    name: name.clone(),
                    modified_at: None,
                    size: None,
                })
                .collect();
            Ok(AiProviderModelsResponse {
                provider,
                reachable: settings.api_key_present,
                models,
                selected_model,
                message: if settings.api_key_present {
                    "Cloud provider key present. Model names are user-configured.".to_string()
                } else {
                    "Add your API key to enable this cloud provider.".to_string()
                },
            })
        }
    }
}

#[tauri::command]
fn ai_chat(request: AiChatRequest) -> CommandResult<AiChatResponse> {
    let connection = open_project_database(&request.project_path)?;
    if cloud_provider(&request.provider) {
        let settings = provider_settings(&connection, request.provider)?;
        if settings.disclosure_accepted_at.is_none() {
            return Err(
                "Accept the cloud model disclosure before sending Vault context to this provider."
                    .to_string(),
            );
        }
        if !settings.api_key_present {
            return Err(
                "Add an API key for this cloud provider before sending a Co-Writer request."
                    .to_string(),
            );
        }
    }

    match request.provider {
        AiProviderKind::Ollama => chat_ollama(&request),
        AiProviderKind::OpenAi | AiProviderKind::OpenAiCompatible => {
            chat_openai_compatible(&connection, &request)
        }
        AiProviderKind::Anthropic => chat_anthropic(&connection, &request),
        AiProviderKind::GoogleAiStudio => chat_google(&connection, &request),
    }
}

#[tauri::command]
fn chat_with_vault(request: ChatWithVaultRequest) -> CommandResult<ChatWithVaultResponse> {
    let connection = open_project_database(&request.project_path)?;
    crate::llm::chat_with_vault(&connection, &request)
}

#[tauri::command]
fn export_item_markdown(request: ExportItemRequest) -> CommandResult<ExportResponse> {
    let project_dir = validate_project_dir(PathBuf::from(&request.project_path))?;
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
fn export_project_json(project_path: String) -> CommandResult<ExportResponse> {
    let project_dir = validate_project_dir(PathBuf::from(&project_path))?;
    let metadata = read_metadata(&project_dir)?;
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
fn export_vault_items_json(project_path: String) -> CommandResult<ExportResponse> {
    let project_dir = validate_project_dir(PathBuf::from(&project_path))?;
    let metadata = read_metadata(&project_dir)?;
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
fn external_vault_parse(path: Option<String>) -> CommandResult<ExternalVaultStructure> {
    parse_external_vault(path)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            app_ping,
            project_create,
            project_open,
            project_get_metadata,
            db_init,
            db_get_vault_tree,
            db_get_item,
            db_update_item,
            db_import_text,
            db_archive_item,
            db_delete_item,
            db_create_vault_node,
            db_search_chunks,
            wards_list,
            wards_add,
            wards_remove,
            wards_scan,
            ai_get_provider_settings,
            ai_save_provider_settings,
            ai_set_api_key,
            ai_delete_api_key,
            ai_accept_cloud_disclosure,
            ai_select_provider,
            ai_list_models,
            ai_chat,
            chat_with_vault,
            ollama_get_status,
            ollama_select_model,
            ollama_chat,
            export_item_markdown,
            export_vault_items_json,
            external_vault_parse,
            export_project_json
        ])
        .run(tauri::generate_context!())
        .expect("error while running Grimoire");
}

fn default_projects_dir() -> CommandResult<PathBuf> {
    let home = env::var_os("HOME").ok_or("Could not resolve HOME for project storage")?;
    Ok(PathBuf::from(home)
        .join("Documents")
        .join("Grimoire Projects"))
}

fn project_folder_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter_map(|character| {
            if character.is_ascii_alphanumeric()
                || character == ' '
                || character == '-'
                || character == '_'
            {
                Some(character)
            } else {
                None
            }
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

fn validate_project_dir(project_dir: PathBuf) -> CommandResult<PathBuf> {
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

fn load_or_create_metadata(project_dir: &Path, name: &str) -> CommandResult<ProjectMetadata> {
    let metadata_path = project_dir.join(METADATA_FILE);
    if metadata_path.exists() {
        return read_metadata(project_dir);
    }

    let now = timestamp();
    let metadata = ProjectMetadata {
        name: name.trim().to_string(),
        app_version: APP_VERSION.to_string(),
        schema_version: SCHEMA_VERSION,
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

fn read_metadata(project_dir: &Path) -> CommandResult<ProjectMetadata> {
    let metadata_path = project_dir.join(METADATA_FILE);
    let raw = fs::read_to_string(&metadata_path).map_err(|error| {
        format!(
            "Could not read project metadata at {}: {error}",
            metadata_path.display()
        )
    })?;

    serde_json::from_str(&raw).map_err(|error| {
        format!(
            "Could not parse project metadata at {}: {error}",
            metadata_path.display()
        )
    })
}

fn write_metadata(metadata: &ProjectMetadata) -> CommandResult<()> {
    let raw = serde_json::to_string_pretty(metadata)
        .map_err(|error| format!("Could not serialize project metadata: {error}"))?;
    fs::write(&metadata.project_path_file(), raw).map_err(|error| {
        format!(
            "Could not write project metadata at {}: {error}",
            metadata.project_path_file().display()
        )
    })
}

impl ProjectMetadata {
    fn project_path_file(&self) -> PathBuf {
        PathBuf::from(&self.project_path).join(METADATA_FILE)
    }
}

fn open_project_database(project_path: &str) -> CommandResult<Connection> {
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

fn initialise_database(metadata: &ProjectMetadata, seed_demo_data: bool) -> CommandResult<()> {
    let mut connection = Connection::open(&metadata.database_path)
        .map_err(|error| format!("Could not open SQLite database: {error}"))?;

    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| format!("Could not enable SQLite foreign keys: {error}"))?;

    run_migrations(&mut connection)?;
    upsert_project_metadata(&connection, metadata)?;
    seed_default_banned_words(&connection)?;

    if seed_demo_data {
        seed_vault_demo_data(&mut connection)?;
    }

    Ok(())
}

fn run_migrations(connection: &mut Connection) -> CommandResult<()> {
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

fn upsert_project_metadata(
    connection: &Connection,
    metadata: &ProjectMetadata,
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

fn seed_vault_demo_data(connection: &mut Connection) -> CommandResult<()> {
    let existing_wings: i64 = connection
        .query_row("SELECT COUNT(*) FROM wings", [], |row| row.get(0))
        .map_err(|error| format!("Could not inspect Vault seed data: {error}"))?;

    if existing_wings > 0 {
        return Ok(());
    }

    let now = timestamp();
    let transaction = connection
        .transaction()
        .map_err(|error| format!("Could not start Vault seed transaction: {error}"))?;

    transaction
        .execute(
            "INSERT INTO wings (id, name, description, sort_order, created_at, updated_at) VALUES (?1, ?2, ?3, 0, ?4, ?4)",
            params!["wing_novel", "The Novel", "Demo writing project", now],
        )
        .map_err(|error| format!("Could not seed demo wing: {error}"))?;

    let halls = [
        ("hall_characters", "wing_novel", "Characters", 0),
        ("hall_world", "wing_novel", "World", 1),
        ("hall_drafts", "wing_novel", "Drafts", 2),
    ];
    for (id, wing_id, name, sort_order) in halls {
        transaction
            .execute(
                "INSERT INTO halls (id, wing_id, name, sort_order, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![id, wing_id, name, sort_order, now],
            )
            .map_err(|error| format!("Could not seed demo hall: {error}"))?;
    }

    let rooms = [
        ("room_protagonists", "hall_characters", "Protagonists", 0),
        ("room_cities", "hall_world", "Cities", 0),
        ("room_act_one", "hall_drafts", "Act One", 0),
    ];
    for (id, hall_id, name, sort_order) in rooms {
        transaction
            .execute(
                "INSERT INTO rooms (id, hall_id, name, sort_order, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![id, hall_id, name, sort_order, now],
            )
            .map_err(|error| format!("Could not seed demo room: {error}"))?;
    }

    let drawers = [
        ("drawer_main_cast", "room_protagonists", "Main Cast", 0),
        (
            "drawer_northern_cities",
            "room_cities",
            "Northern Cities",
            0,
        ),
        (
            "drawer_opening_sequence",
            "room_act_one",
            "Opening Sequence",
            0,
        ),
    ];
    for (id, room_id, name, sort_order) in drawers {
        transaction
            .execute(
                "INSERT INTO drawers (id, room_id, name, sort_order, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![id, room_id, name, sort_order, now],
            )
            .map_err(|error| format!("Could not seed demo drawer: {error}"))?;
    }

    let items = [
        (
            "item_mara",
            "drawer_main_cast",
            "Mara Thorne",
            "character",
            "Mara speaks like someone who learned early that every word can be used against her. When she is angry, her sentences become shorter. When she is afraid, she becomes polite.",
            "The Novel / Characters / Protagonists / Main Cast / Mara Thorne",
            0,
        ),
        (
            "item_vel_ashen",
            "drawer_northern_cities",
            "Vel Ashen",
            "location",
            "Vel Ashen smells of river silt, old stone, bridge smoke, and wet iron at dawn. The city is not romantic to Mara. It is familiar, dangerous, and useful.",
            "The Novel / World / Cities / Northern Cities / Vel Ashen",
            1,
        ),
        (
            "item_chapter_01",
            "drawer_opening_sequence",
            "Chapter 01: The Bell Beneath the River",
            "chapter",
            "The bell rang below the river before anyone in Vel Ashen admitted they could hear it. Mara counted the sound by instinct and kept walking.",
            "The Novel / Drafts / Act One / Opening Sequence / Chapter 01: The Bell Beneath the River",
            2,
        ),
    ];

    for (id, drawer_id, title, item_type, content, vault_path, sort_order) in items {
        let word_count = count_words(content);
        transaction
            .execute(
                r#"
                INSERT INTO items (
                  id, drawer_id, title, item_type, content, plain_text, word_count,
                  source_kind, sort_order, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6, 'demo', ?7, ?8, ?8)
                "#,
                params![id, drawer_id, title, item_type, content, word_count, sort_order, now],
            )
            .map_err(|error| format!("Could not seed demo item: {error}"))?;

        let chunk_id = format!("{id}_chunk_0");
        transaction
            .execute(
                r#"
                INSERT INTO item_chunks (
                  id, item_id, chunk_index, text, word_count, start_offset,
                  end_offset, created_at, updated_at
                )
                VALUES (?1, ?2, 0, ?3, ?4, 0, ?5, ?6, ?6)
                "#,
                params![chunk_id, id, content, word_count, content.len() as i64, now],
            )
            .map_err(|error| format!("Could not seed demo chunk: {error}"))?;

        transaction
            .execute(
                r#"
                INSERT INTO item_chunks_fts (
                  chunk_id, item_id, title, item_type, vault_path, text
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![chunk_id, id, title, item_type, vault_path, content],
            )
            .map_err(|error| format!("Could not seed demo search index: {error}"))?;
    }

    transaction
        .commit()
        .map_err(|error| format!("Could not commit Vault seed data: {error}"))?;

    Ok(())
}

fn count_words(text: &str) -> i64 {
    text.split_whitespace().count() as i64
}

fn read_vault_tree(connection: &Connection) -> CommandResult<VaultTreeResponse> {
    let wing_rows = collect_named_rows(
        connection,
        "SELECT id, name, description FROM wings ORDER BY sort_order, name",
        [],
    )?;

    let mut item_count = 0;
    let mut wings = Vec::new();

    for (wing_id, wing_name, wing_description) in wing_rows {
        let halls = read_halls(connection, &wing_id, &wing_name, &mut item_count)?;
        wings.push(VaultWingNode {
            id: wing_id,
            name: wing_name,
            description: wing_description,
            halls,
        });
    }

    Ok(VaultTreeResponse { wings, item_count })
}

fn read_halls(
    connection: &Connection,
    wing_id: &str,
    wing_name: &str,
    item_count: &mut usize,
) -> CommandResult<Vec<VaultHallNode>> {
    let hall_rows = collect_named_rows(
        connection,
        "SELECT id, name, description FROM halls WHERE wing_id = ?1 ORDER BY sort_order, name",
        params![wing_id],
    )?;

    let mut halls = Vec::new();
    for (hall_id, hall_name, hall_description) in hall_rows {
        let rooms = read_rooms(connection, &hall_id, wing_name, &hall_name, item_count)?;
        halls.push(VaultHallNode {
            id: hall_id,
            name: hall_name,
            description: hall_description,
            rooms,
        });
    }

    Ok(halls)
}

fn read_rooms(
    connection: &Connection,
    hall_id: &str,
    wing_name: &str,
    hall_name: &str,
    item_count: &mut usize,
) -> CommandResult<Vec<VaultRoomNode>> {
    let room_rows = collect_named_rows(
        connection,
        "SELECT id, name, description FROM rooms WHERE hall_id = ?1 ORDER BY sort_order, name",
        params![hall_id],
    )?;

    let mut rooms = Vec::new();
    for (room_id, room_name, room_description) in room_rows {
        let drawers = read_drawers(
            connection, &room_id, wing_name, hall_name, &room_name, item_count,
        )?;
        rooms.push(VaultRoomNode {
            id: room_id,
            name: room_name,
            description: room_description,
            drawers,
        });
    }

    Ok(rooms)
}

fn read_drawers(
    connection: &Connection,
    room_id: &str,
    wing_name: &str,
    hall_name: &str,
    room_name: &str,
    item_count: &mut usize,
) -> CommandResult<Vec<VaultDrawerNode>> {
    let drawer_rows = collect_named_rows(
        connection,
        "SELECT id, name, description FROM drawers WHERE room_id = ?1 ORDER BY sort_order, name",
        params![room_id],
    )?;

    let mut drawers = Vec::new();
    for (drawer_id, drawer_name, drawer_description) in drawer_rows {
        let items = read_items(
            connection,
            &drawer_id,
            wing_name,
            hall_name,
            room_name,
            &drawer_name,
        )?;
        *item_count += items.len();
        drawers.push(VaultDrawerNode {
            id: drawer_id,
            name: drawer_name,
            description: drawer_description,
            items,
        });
    }

    Ok(drawers)
}

fn read_items(
    connection: &Connection,
    drawer_id: &str,
    wing_name: &str,
    hall_name: &str,
    room_name: &str,
    drawer_name: &str,
) -> CommandResult<Vec<VaultItemNode>> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT id, title, item_type, content, word_count
            FROM items
            WHERE drawer_id = ?1
              AND archived_at IS NULL
            ORDER BY sort_order, title
            "#,
        )
        .map_err(|error| format!("Could not prepare item query: {error}"))?;

    let mapped = statement
        .query_map(params![drawer_id], |row| {
            let title: String = row.get(1)?;
            Ok(VaultItemNode {
                id: row.get(0)?,
                path: format!("{wing_name} / {hall_name} / {room_name} / {drawer_name} / {title}"),
                title,
                item_type: row.get(2)?,
                content: row.get(3)?,
                word_count: row.get(4)?,
            })
        })
        .map_err(|error| format!("Could not query Vault items: {error}"))?;

    let mut items = Vec::new();
    for item in mapped {
        items.push(item.map_err(|error| format!("Could not read Vault item: {error}"))?);
    }

    Ok(items)
}

fn read_item_detail(connection: &Connection, item_id: &str) -> CommandResult<VaultItemDetail> {
    connection
        .query_row(
            r#"
            SELECT
              i.id,
              i.title,
              i.item_type,
              COALESCE(i.content, ''),
              COALESCE(i.plain_text, ''),
              i.word_count,
              i.updated_at,
              w.name,
              h.name,
              r.name,
              d.name
            FROM items i
            JOIN drawers d ON d.id = i.drawer_id
            JOIN rooms r ON r.id = d.room_id
            JOIN halls h ON h.id = r.hall_id
            JOIN wings w ON w.id = h.wing_id
            WHERE i.id = ?1
              AND i.archived_at IS NULL
            "#,
            params![item_id],
            |row| {
                let title: String = row.get(1)?;
                let wing: String = row.get(7)?;
                let hall: String = row.get(8)?;
                let room: String = row.get(9)?;
                let drawer: String = row.get(10)?;
                Ok(VaultItemDetail {
                    id: row.get(0)?,
                    title: title.clone(),
                    item_type: row.get(2)?,
                    content: row.get(3)?,
                    plain_text: row.get(4)?,
                    word_count: row.get(5)?,
                    updated_at: row.get(6)?,
                    path: format!("{wing} / {hall} / {room} / {drawer} / {title}"),
                })
            },
        )
        .map_err(|error| format!("Could not read Canvas item: {error}"))
}

fn item_path(connection: &Connection, item_id: &str, title: &str) -> CommandResult<String> {
    connection
        .query_row(
            r#"
            SELECT w.name, h.name, r.name, d.name
            FROM items i
            JOIN drawers d ON d.id = i.drawer_id
            JOIN rooms r ON r.id = d.room_id
            JOIN halls h ON h.id = r.hall_id
            JOIN wings w ON w.id = h.wing_id
            WHERE i.id = ?1
              AND i.archived_at IS NULL
            "#,
            params![item_id],
            |row| {
                let wing: String = row.get(0)?;
                let hall: String = row.get(1)?;
                let room: String = row.get(2)?;
                let drawer: String = row.get(3)?;
                Ok(format!("{wing} / {hall} / {room} / {drawer} / {title}"))
            },
        )
        .map_err(|error| format!("Could not resolve Vault path: {error}"))
}

fn item_type(connection: &Connection, item_id: &str) -> CommandResult<String> {
    connection
        .query_row(
            "SELECT item_type FROM items WHERE id = ?1 AND archived_at IS NULL",
            params![item_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("Could not resolve item type: {error}"))
}

fn ensure_import_drawer(connection: &Connection) -> CommandResult<String> {
    let now = timestamp();
    connection
        .execute(
            r#"
            INSERT OR IGNORE INTO wings (id, name, description, sort_order, created_at, updated_at)
            VALUES ('wing_imports', 'The Vault', 'Imported writing and notes', 99, ?1, ?1)
            "#,
            params![now],
        )
        .map_err(|error| format!("Could not prepare import wing: {error}"))?;
    connection
        .execute(
            r#"
            INSERT OR IGNORE INTO halls (id, wing_id, name, description, sort_order, created_at, updated_at)
            VALUES ('hall_feed', 'wing_imports', 'Feed', 'Material brought into the Vault', 0, ?1, ?1)
            "#,
            params![now],
        )
        .map_err(|error| format!("Could not prepare import hall: {error}"))?;
    connection
        .execute(
            r#"
            INSERT OR IGNORE INTO rooms (id, hall_id, name, description, sort_order, created_at, updated_at)
            VALUES ('room_imports', 'hall_feed', 'Imports', 'Text and Markdown imports', 0, ?1, ?1)
            "#,
            params![now],
        )
        .map_err(|error| format!("Could not prepare import room: {error}"))?;
    connection
        .execute(
            r#"
            INSERT OR IGNORE INTO drawers (id, room_id, name, description, sort_order, created_at, updated_at)
            VALUES ('drawer_imported_text', 'room_imports', 'Imported Text', 'Newly imported writing', 0, ?1, ?1)
            "#,
            params![now],
        )
        .map_err(|error| format!("Could not prepare import drawer: {error}"))?;
    Ok("drawer_imported_text".to_string())
}

fn ensure_hierarchy_node(
    connection: &Connection,
    table: &str,
    id: &str,
    label: &str,
) -> CommandResult<()> {
    let query = format!("SELECT COUNT(*) FROM {table} WHERE id = ?1");
    let count: i64 = connection
        .query_row(&query, params![id], |row| row.get(0))
        .map_err(|error| format!("Could not verify parent {label}: {error}"))?;
    if count == 0 {
        return Err(format!("Parent {label} not found."));
    }
    Ok(())
}

fn next_sort_order(
    connection: &Connection,
    table: &str,
    parent_column: &str,
    parent_id: &str,
) -> CommandResult<i64> {
    let query =
        format!("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM {table} WHERE {parent_column} = ?1");
    connection
        .query_row(&query, params![parent_id], |row| row.get(0))
        .map_err(|error| format!("Could not calculate sort order: {error}"))
}

fn clear_item_chunks(connection: &Connection, item_id: &str) -> CommandResult<()> {
    connection
        .execute(
            "DELETE FROM item_chunks_fts WHERE item_id = ?1",
            params![item_id],
        )
        .map_err(|error| format!("Could not clear search index: {error}"))?;
    connection
        .execute(
            "DELETE FROM item_chunks WHERE item_id = ?1",
            params![item_id],
        )
        .map_err(|error| format!("Could not clear item chunks: {error}"))?;
    Ok(())
}

fn sync_item_chunks(
    connection: &Connection,
    item_id: &str,
    title: &str,
    item_type: &str,
    vault_path: &str,
    text: &str,
) -> CommandResult<usize> {
    clear_item_chunks(connection, item_id)?;
    let chunks = chunk_text(text, 240);
    let now = timestamp();
    for (index, chunk) in chunks.iter().enumerate() {
        let chunk_id = format!("{item_id}_chunk_{index}");
        connection
            .execute(
                r#"
                INSERT INTO item_chunks (
                  id, item_id, chunk_index, text, word_count, start_offset,
                  end_offset, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, ?6, ?6)
                "#,
                params![
                    chunk_id,
                    item_id,
                    index as i64,
                    chunk,
                    count_words(chunk),
                    now
                ],
            )
            .map_err(|error| format!("Could not write item chunk: {error}"))?;
        connection
            .execute(
                r#"
                INSERT INTO item_chunks_fts (
                  chunk_id, item_id, title, item_type, vault_path, text
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![chunk_id, item_id, title, item_type, vault_path, chunk],
            )
            .map_err(|error| format!("Could not update search index: {error}"))?;
    }

    Ok(chunks.len())
}

fn chunk_text(text: &str, max_words: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return Vec::new();
    }

    words
        .chunks(max_words.max(1))
        .map(|chunk| chunk.join(" "))
        .collect()
}

fn normalize_text(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut blank_lines = 0;
    let mut lines = Vec::new();
    for line in normalized.lines() {
        let trimmed_end = line.trim_end();
        if trimmed_end.trim().is_empty() {
            blank_lines += 1;
            if blank_lines <= 2 {
                lines.push(String::new());
            }
        } else {
            blank_lines = 0;
            lines.push(trimmed_end.to_string());
        }
    }

    lines.join("\n").trim().to_string()
}

fn import_progress_labels() -> Vec<String> {
    vec![
        "Reading the bones".to_string(),
        "Distilling word essence".to_string(),
        "Mapping canon traces".to_string(),
        "Stocking the Vault".to_string(),
    ]
}

fn fts_query(query: &str) -> CommandResult<String> {
    let tokens = search_tokens(query);
    if tokens.is_empty() {
        return Err("Search needs at least one word or number.".to_string());
    }

    Ok(tokens
        .into_iter()
        .take(8)
        .map(|token| format!("{token}*"))
        .collect::<Vec<_>>()
        .join(" "))
}

fn fts_query_broad(query: &str) -> CommandResult<String> {
    let filtered = search_tokens(query)
        .into_iter()
        .filter(|token| !vault_recall_stopword(token))
        .collect::<Vec<_>>();
    let tokens = if filtered.is_empty() {
        search_tokens(query)
    } else {
        filtered
    };

    if tokens.is_empty() {
        return Err("Search needs at least one word or number.".to_string());
    }

    Ok(tokens
        .into_iter()
        .take(12)
        .map(|token| format!("{token}*"))
        .collect::<Vec<_>>()
        .join(" OR "))
}

fn search_tokens(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for character in query.chars() {
        if character.is_alphanumeric() || character == '_' {
            current.push(character);
        } else if !current.is_empty() {
            tokens.push(current.to_lowercase());
            current.clear();
        }
    }
    if !current.is_empty() {
        tokens.push(current.to_lowercase());
    }

    tokens
}

fn vault_recall_stopword(token: &str) -> bool {
    matches!(
        token,
        "a" | "an"
            | "and"
            | "are"
            | "as"
            | "at"
            | "about"
            | "be"
            | "because"
            | "but"
            | "by"
            | "can"
            | "could"
            | "do"
            | "does"
            | "for"
            | "from"
            | "have"
            | "how"
            | "i"
            | "in"
            | "is"
            | "it"
            | "its"
            | "me"
            | "of"
            | "on"
            | "or"
            | "please"
            | "should"
            | "tell"
            | "that"
            | "the"
            | "their"
            | "there"
            | "this"
            | "to"
            | "was"
            | "what"
            | "when"
            | "where"
            | "which"
            | "who"
            | "why"
            | "with"
            | "would"
            | "you"
    )
}

fn confidence_for_score(score: f64) -> String {
    if score >= 8.0 {
        "high".to_string()
    } else if score >= 3.0 {
        "medium".to_string()
    } else if score > 0.0 {
        "low".to_string()
    } else {
        "none".to_string()
    }
}

fn aggregate_confidence(results: &[SearchChunkResult]) -> String {
    results
        .first()
        .map(|result| result.confidence.clone())
        .unwrap_or_else(|| "none".to_string())
}

fn get_setting(connection: &Connection, key: &str) -> CommandResult<Option<String>> {
    let result = connection.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    );

    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(format!("Could not read setting {key}: {error}")),
    }
}

fn get_active_provider(connection: &Connection) -> CommandResult<AiProviderKind> {
    Ok(
        match get_setting(connection, "ai.activeProvider")?.as_deref() {
            Some("openAi") => AiProviderKind::OpenAi,
            Some("openAiCompatible") => AiProviderKind::OpenAiCompatible,
            Some("anthropic") => AiProviderKind::Anthropic,
            Some("googleAiStudio") => AiProviderKind::GoogleAiStudio,
            _ => AiProviderKind::Ollama,
        },
    )
}

fn provider_setting_key(provider: AiProviderKind, field: &str) -> String {
    format!("ai.provider.{}.{}", provider.as_key(), field)
}

fn provider_settings(
    connection: &Connection,
    provider: AiProviderKind,
) -> CommandResult<AiProviderSettings> {
    let selected_model = get_setting(connection, &provider_setting_key(provider, "selectedModel"))?
        .or_else(|| provider.default_model().map(ToString::to_string));
    let base_url = get_setting(connection, &provider_setting_key(provider, "baseUrl"))?
        .or_else(|| provider.default_base_url().map(ToString::to_string));
    let disclosure_accepted_at = get_setting(
        connection,
        &provider_setting_key(provider, "disclosureAcceptedAt"),
    )?;
    let api_key_present = if cloud_provider(&provider) {
        get_setting(connection, &provider_setting_key(provider, "apiKeyPresent"))?.as_deref()
            == Some("true")
    } else {
        false
    };

    Ok(AiProviderSettings {
        provider,
        display_name: provider.display_name().to_string(),
        base_url,
        selected_model,
        api_key_present,
        disclosure_accepted_at,
        enabled: provider == AiProviderKind::Ollama || api_key_present,
    })
}

fn list_ollama_models(connection: &Connection) -> CommandResult<AiProviderModelsResponse> {
    let base_url = "http://127.0.0.1:11434";
    match fetch_ollama_models(base_url) {
        Ok(models) => {
            let previous = get_setting(
                connection,
                &provider_setting_key(AiProviderKind::Ollama, "selectedModel"),
            )?
            .or_else(|| {
                get_setting(connection, "ollama.selectedModel")
                    .ok()
                    .flatten()
            });
            let model_names: Vec<String> = models.iter().map(|model| model.name.clone()).collect();
            let selected_model = select_ollama_model(previous, &model_names);
            if let Some(model) = selected_model.as_deref().filter(|_| model_names.len() == 1) {
                set_setting(
                    connection,
                    &provider_setting_key(AiProviderKind::Ollama, "selectedModel"),
                    model,
                )?;
            }
            let message = if models.is_empty() {
                "Ollama is running, but no local models are installed. Install one with `ollama pull <model>` and refresh models.".to_string()
            } else if selected_model.is_some() {
                "Ollama model ready.".to_string()
            } else {
                "Ollama found multiple local models. Choose one to enable Co-Writer requests."
                    .to_string()
            };

            Ok(AiProviderModelsResponse {
                provider: AiProviderKind::Ollama,
                reachable: true,
                models,
                selected_model,
                message,
            })
        }
        Err(message) => Ok(AiProviderModelsResponse {
            provider: AiProviderKind::Ollama,
            reachable: false,
            models: Vec::new(),
            selected_model: None,
            message,
        }),
    }
}

fn select_ollama_model(previous: Option<String>, model_names: &[String]) -> Option<String> {
    if let Some(model) = previous.filter(|model| model_names.contains(model)) {
        Some(model)
    } else if model_names.len() == 1 {
        model_names.first().cloned()
    } else {
        None
    }
}

fn set_setting(connection: &Connection, key: &str, value: &str) -> CommandResult<()> {
    connection
        .execute(
            r#"
            INSERT INTO settings (key, value, updated_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
            "#,
            params![key, value, timestamp()],
        )
        .map_err(|error| format!("Could not write setting {key}: {error}"))?;
    Ok(())
}

fn secret_account(project_path: &str, provider: AiProviderKind) -> String {
    let mut project_hash: u64 = 1469598103934665603;
    for byte in project_path.as_bytes() {
        project_hash ^= *byte as u64;
        project_hash = project_hash.wrapping_mul(1099511628211);
    }
    format!("{:016x}-{}", project_hash, provider.as_key())
}

fn secret_service(provider: AiProviderKind) -> String {
    format!("com.witchdaddylabs.grimoire.{}", provider.as_key())
}

fn secret_entry(project_path: &str, provider: AiProviderKind) -> CommandResult<Entry> {
    Entry::new(
        &secret_service(provider),
        &secret_account(project_path, provider),
    )
    .map_err(|error| format!("Could not open the secure credential store: {error}"))
}

fn set_api_key_secret(
    project_path: &str,
    provider: AiProviderKind,
    api_key: &str,
) -> CommandResult<()> {
    secret_entry(project_path, provider)?
        .set_password(api_key)
        .map_err(|error| format!("Could not save provider API key in the credential store: {error}"))
}

fn get_api_key_secret(
    project_path: &str,
    provider: AiProviderKind,
) -> CommandResult<Option<String>> {
    match secret_entry(project_path, provider)?.get_password() {
        Ok(key) => {
            let key = key.trim().to_string();
            if key.is_empty() {
                Ok(None)
            } else {
                Ok(Some(key))
            }
        }
        Err(KeyringError::NoEntry) => Ok(None),
        Err(error) => Err(format!("Could not read provider API key: {error}")),
    }
}

fn delete_api_key_secret(project_path: &str, provider: AiProviderKind) -> CommandResult<()> {
    match secret_entry(project_path, provider)?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(error) => Err(format!("Could not delete provider API key: {error}")),
    }
}

fn fetch_ollama_models(base_url: &str) -> Result<Vec<AiModelInfo>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|error| format!("Could not prepare Ollama client: {error}"))?;
    let response: Value = client
        .get(format!("{base_url}/api/tags"))
        .send()
        .map_err(|error| format!("Ollama is not reachable at {base_url}: {error}"))?
        .json()
        .map_err(|error| format!("Could not read Ollama model list: {error}"))?;
    let models = response
        .get("models")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let name = item.get("name")?.as_str()?.to_string();
                    Some(AiModelInfo {
                        name,
                        modified_at: item
                            .get("modified_at")
                            .and_then(Value::as_str)
                            .map(ToString::to_string),
                        size: item.get("size").and_then(Value::as_i64),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(models)
}

fn chat_ollama(request: &AiChatRequest) -> CommandResult<AiChatResponse> {
    let content = if request.grounded_context.trim().is_empty() {
        request.prompt.clone()
    } else {
        format!(
            "{}\n\nUser request:\n{}",
            request.grounded_context, request.prompt
        )
    };
    let client = http_client(Duration::from_secs(120))?;
    let payload = json!({
        "model": request.model,
        "stream": false,
        "messages": [
            {
                "role": "user",
                "content": content
            }
        ]
    });
    let response: Value = client
        .post("http://127.0.0.1:11434/api/chat")
        .json(&payload)
        .send()
        .map_err(|error| format!("Could not reach Ollama chat endpoint: {error}"))?
        .json()
        .map_err(|error| format!("Could not read Ollama chat response: {error}"))?;
    let text = response
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if text.is_empty() {
        return Err("Ollama returned an empty response.".to_string());
    }
    Ok(AiChatResponse {
        provider: AiProviderKind::Ollama,
        model: request.model.clone(),
        text,
        request_id: None,
        input_tokens: None,
        output_tokens: None,
    })
}

fn chat_openai_compatible(
    connection: &Connection,
    request: &AiChatRequest,
) -> CommandResult<AiChatResponse> {
    let api_key = get_api_key_secret(&request.project_path, request.provider)?
        .ok_or("Add an API key for this cloud provider before sending a Co-Writer request.")?;
    let base_url = provider_settings(connection, request.provider)?
        .base_url
        .ok_or("Set a base URL for this OpenAI-compatible provider.")?;
    let url = if request.provider == AiProviderKind::OpenAi {
        format!("{}/v1/chat/completions", base_url.trim_end_matches('/'))
    } else {
        openai_compatible_url(&base_url)
    };
    let client = http_client(Duration::from_secs(120))?;
    let payload = json!({
        "model": request.model,
        "messages": [
            {
                "role": "system",
                "content": "Use the provided Grimoire context. Cite local sources when present. Do not claim access to unavailable context."
            },
            {
                "role": "user",
                "content": format!("{}\n\nUser request:\n{}", request.grounded_context, request.prompt)
            }
        ]
    });
    let response = client
        .post(url)
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .map_err(|error| format!("Cloud provider request failed: {error}"))?;
    let status = response.status().as_u16();
    let raw = response
        .text()
        .map_err(|error| format!("Could not read cloud provider response: {error}"))?;
    if !(200..300).contains(&status) {
        return Err(cloud_http_error("Cloud provider", status));
    }
    let response: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("Could not parse cloud provider response: {error}"))?;
    let text = openai_chat_text(&response)
        .ok_or("Cloud provider returned an empty response.".to_string())?;
    Ok(AiChatResponse {
        provider: request.provider,
        model: request.model.clone(),
        text,
        request_id: response
            .get("id")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        input_tokens: response
            .get("usage")
            .and_then(|usage| usage.get("prompt_tokens"))
            .and_then(Value::as_i64),
        output_tokens: response
            .get("usage")
            .and_then(|usage| usage.get("completion_tokens"))
            .and_then(Value::as_i64),
    })
}

fn chat_anthropic(
    connection: &Connection,
    request: &AiChatRequest,
) -> CommandResult<AiChatResponse> {
    let api_key = get_api_key_secret(&request.project_path, request.provider)?
        .ok_or("Add an API key for Anthropic before sending a Co-Writer request.")?;
    let base_url = provider_settings(connection, request.provider)?
        .base_url
        .unwrap_or_else(|| "https://api.anthropic.com".to_string());
    let client = http_client(Duration::from_secs(120))?;
    let payload = json!({
        "model": request.model,
        "max_tokens": 1200,
        "system": "Use the provided Grimoire context. Cite local sources when present. Do not claim access to unavailable context.",
        "messages": [
            {
                "role": "user",
                "content": format!("{}\n\nUser request:\n{}", request.grounded_context, request.prompt)
            }
        ]
    });
    let mut request_builder =
        client.post(format!("{}/v1/messages", base_url.trim_end_matches('/')));
    for (name, value) in anthropic_headers(&api_key) {
        request_builder = request_builder.header(name, value);
    }
    let response = request_builder
        .json(&payload)
        .send()
        .map_err(|error| format!("Anthropic request failed: {error}"))?;
    let status = response.status().as_u16();
    let raw = response
        .text()
        .map_err(|error| format!("Could not read Anthropic response: {error}"))?;
    if !(200..300).contains(&status) {
        return Err(cloud_http_error("Anthropic", status));
    }
    let response: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("Could not parse Anthropic response: {error}"))?;
    let text = anthropic_chat_text(&response)
        .ok_or("Anthropic returned an empty response.".to_string())?;
    Ok(AiChatResponse {
        provider: request.provider,
        model: request.model.clone(),
        text,
        request_id: response
            .get("id")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        input_tokens: response
            .get("usage")
            .and_then(|usage| usage.get("input_tokens"))
            .and_then(Value::as_i64),
        output_tokens: response
            .get("usage")
            .and_then(|usage| usage.get("output_tokens"))
            .and_then(Value::as_i64),
    })
}

fn chat_google(connection: &Connection, request: &AiChatRequest) -> CommandResult<AiChatResponse> {
    let api_key = get_api_key_secret(&request.project_path, request.provider)?
        .ok_or("Add an API key for Google AI Studio before sending a Co-Writer request.")?;
    let base_url = provider_settings(connection, request.provider)?
        .base_url
        .unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string());
    let client = http_client(Duration::from_secs(120))?;
    let payload = json!({
        "contents": [
            {
                "role": "user",
                "parts": [
                    {
                        "text": format!("Use the provided Grimoire context. Cite local sources when present. Do not claim access to unavailable context.\n\n{}\n\nUser request:\n{}", request.grounded_context, request.prompt)
                    }
                ]
            }
        ]
    });
    let url = if base_url.trim_end_matches('/') == "https://generativelanguage.googleapis.com" {
        gemini_generate_content_url(&request.model)
    } else {
        format!(
            "{}/v1beta/models/{}:generateContent",
            base_url.trim_end_matches('/'),
            request.model
        )
    };
    let response = client
        .post(url)
        .header("x-goog-api-key", api_key)
        .json(&payload)
        .send()
        .map_err(|error| format!("Google AI Studio request failed: {error}"))?;
    let status = response.status().as_u16();
    let raw = response
        .text()
        .map_err(|error| format!("Could not read Google AI Studio response: {error}"))?;
    if !(200..300).contains(&status) {
        return Err(cloud_http_error("Google AI Studio", status));
    }
    let response: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("Could not parse Google AI Studio response: {error}"))?;
    let text = gemini_chat_text(&response)
        .ok_or("Google AI Studio returned an empty response.".to_string())?;
    Ok(AiChatResponse {
        provider: request.provider,
        model: request.model.clone(),
        text,
        request_id: None,
        input_tokens: response
            .get("usageMetadata")
            .and_then(|usage| usage.get("promptTokenCount"))
            .and_then(Value::as_i64),
        output_tokens: response
            .get("usageMetadata")
            .and_then(|usage| usage.get("candidatesTokenCount"))
            .and_then(Value::as_i64),
    })
}

fn http_client(timeout: Duration) -> CommandResult<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|error| format!("Could not prepare HTTP client: {error}"))
}

fn cloud_http_error(provider: &str, status: u16) -> String {
    match status {
        401 | 403 => format!("{provider} rejected the API key or account permissions."),
        404 => format!("{provider} could not find that model or endpoint."),
        408 | 429 => format!("{provider} is rate limited, over quota, or timed out."),
        500..=599 => format!("{provider} returned a temporary server error ({status})."),
        _ => format!("{provider} request failed with HTTP status {status}."),
    }
}

fn openai_compatible_url(base_url: &str) -> String {
    format!("{}/v1/chat/completions", base_url.trim_end_matches('/'))
}

fn openai_chat_text(response: &Value) -> Option<String> {
    response
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
}

fn anthropic_headers(api_key: &str) -> Vec<(String, String)> {
    vec![
        ("x-api-key".to_string(), api_key.to_string()),
        ("anthropic-version".to_string(), "2023-06-01".to_string()),
        ("content-type".to_string(), "application/json".to_string()),
    ]
}

fn anthropic_chat_text(response: &Value) -> Option<String> {
    response
        .get("content")
        .and_then(Value::as_array)
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|block| block.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn gemini_generate_content_url(model: &str) -> String {
    format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent")
}

fn gemini_chat_text(response: &Value) -> Option<String> {
    response
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|candidates| candidates.first())
        .and_then(|candidate| candidate.get("content"))
        .and_then(|content| content.get("parts"))
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
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

fn collect_named_rows<P>(
    connection: &Connection,
    query: &str,
    params: P,
) -> CommandResult<Vec<(String, String, Option<String>)>>
where
    P: rusqlite::Params,
{
    let mut statement = connection
        .prepare(query)
        .map_err(|error| format!("Could not prepare Vault tree query: {error}"))?;
    let mapped = statement
        .query_map(params, |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|error| format!("Could not query Vault tree: {error}"))?;

    let mut rows = Vec::new();
    for row in mapped {
        rows.push(row.map_err(|error| format!("Could not read Vault tree row: {error}"))?);
    }

    Ok(rows)
}

fn timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    seconds.to_string()
}

fn timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{cloud_provider, AiProviderKind, CLOUD_DISCLOSURE_COPY};

    #[test]
    fn normalize_text_trims_crlf_and_extra_blank_lines() {
        assert_eq!(
            normalize_text(" hello \r\n\r\n\r\nworld  "),
            "hello\n\n\nworld"
        );
    }

    #[test]
    fn chunk_text_splits_by_word_limit() {
        let chunks = chunk_text("one two three four five", 2);
        assert_eq!(chunks, vec!["one two", "three four", "five"]);
    }

    #[test]
    fn import_word_limit_matches_release_copy() {
        assert_eq!(MAX_IMPORT_WORDS, 10_000);
    }

    #[test]
    fn fts_query_sanitizes_and_suffixes_terms() {
        assert_eq!(fts_query("Mara's bell!").unwrap(), "mara* s* bell*");
    }

    #[test]
    fn broad_fts_query_uses_recall_terms() {
        assert_eq!(
            fts_query_broad("What is the secret name from the other file?").unwrap(),
            "secret* OR name* OR other* OR file*"
        );
    }

    #[test]
    fn ward_scan_counts_hits_and_blocks() {
        let words = vec![
            BannedWord {
                id: "one".to_string(),
                value: "very".to_string(),
                severity: "warn".to_string(),
                is_default: true,
            },
            BannedWord {
                id: "two".to_string(),
                value: "forbidden".to_string(),
                severity: "block".to_string(),
                is_default: false,
            },
        ];
        let scan = scan_wards(&words, "Very very forbidden");
        assert_eq!(scan.hits.len(), 2);
        assert!(scan.has_blocking_hits);
    }

    #[test]
    fn sanitize_filename_keeps_safe_name() {
        assert_eq!(
            sanitize_filename("Chapter 01: Bell/Bones"),
            "Chapter-01_-Bell_Bones"
        );
        assert_eq!(sanitize_filename("///"), "untitled");
    }

    #[test]
    fn cloud_provider_detection_matches_local_first_rule() {
        assert!(!cloud_provider(&AiProviderKind::Ollama));
        assert!(cloud_provider(&AiProviderKind::OpenAi));
    }

    #[test]
    fn disclosure_copy_names_privacy_policy() {
        assert!(CLOUD_DISCLOSURE_COPY.contains("privacy policy"));
    }

    #[test]
    fn provider_url_helpers_are_stable() {
        assert_eq!(
            openai_compatible_url("https://example.test/"),
            "https://example.test/v1/chat/completions"
        );
        assert!(anthropic_headers("test-key")
            .iter()
            .any(|(name, value)| name == "anthropic-version" && value == "2023-06-01"));
        assert_eq!(
            gemini_generate_content_url("gemini-2.5-flash"),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent"
        );
    }

    #[test]
    fn ollama_model_selection_handles_zero_one_and_many_models() {
        let no_models: Vec<String> = Vec::new();
        assert_eq!(select_ollama_model(None, &no_models), None);

        let one_model = vec!["gemma4:e4b".to_string()];
        assert_eq!(
            select_ollama_model(None, &one_model),
            Some("gemma4:e4b".to_string())
        );

        let many_models = vec!["gemma4:e4b".to_string(), "ministral-3:8b".to_string()];
        assert_eq!(select_ollama_model(None, &many_models), None);
        assert_eq!(
            select_ollama_model(Some("ministral-3:8b".to_string()), &many_models),
            Some("ministral-3:8b".to_string())
        );
        assert_eq!(
            select_ollama_model(Some("missing-model".to_string()), &many_models),
            None
        );
    }

    #[test]
    fn provider_settings_use_stored_key_presence_without_keychain_lookup() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                r#"
                CREATE TABLE settings (
                  key TEXT PRIMARY KEY,
                  value TEXT NOT NULL,
                  updated_at TEXT NOT NULL
                );
                INSERT INTO settings (key, value, updated_at)
                VALUES ('ai.provider.openAi.apiKeyPresent', 'true', 'test');
                "#,
            )
            .unwrap();

        let settings = provider_settings(&connection, AiProviderKind::OpenAi).unwrap();
        assert!(settings.api_key_present);
    }

    #[test]
    fn cloud_response_parsers_extract_text() {
        let openai_response = json!({
            "choices": [
                {
                    "message": {
                        "content": "  OpenAI answer.  "
                    }
                }
            ]
        });
        assert_eq!(
            openai_chat_text(&openai_response),
            Some("OpenAI answer.".to_string())
        );

        let anthropic_response = json!({
            "content": [
                {
                    "type": "text",
                    "text": "First block."
                },
                {
                    "type": "text",
                    "text": "Second block."
                }
            ]
        });
        assert_eq!(
            anthropic_chat_text(&anthropic_response),
            Some("First block.\nSecond block.".to_string())
        );

        let gemini_response = json!({
            "candidates": [
                {
                    "content": {
                        "parts": [
                            {
                                "text": "Gemini answer."
                            }
                        ]
                    }
                }
            ]
        });
        assert_eq!(
            gemini_chat_text(&gemini_response),
            Some("Gemini answer.".to_string())
        );
    }

    #[test]
    fn cloud_response_parsers_reject_empty_text() {
        assert_eq!(openai_chat_text(&json!({"choices": []})), None);
        assert_eq!(anthropic_chat_text(&json!({"content": []})), None);
        assert_eq!(gemini_chat_text(&json!({"candidates": []})), None);
    }
}

const INITIAL_SCHEMA: &str = r#"
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
