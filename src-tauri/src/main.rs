#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod commands;
mod db;
mod errors;
mod external_vault;
mod helpers;
mod llm;
mod models;
mod settings;

use commands::{
    ai as ai_cmds,
    db_init as db_init_cmds,
    export as export_cmds,
    ollama as ollama_cmds,
    project as project_cmds,
    search as search_cmds,
    vault as vault_cmds,
    wards as wards_cmds,
};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            project_cmds::app_ping,
            project_cmds::project_create,
            project_cmds::project_open,
            project_cmds::project_get_metadata,
            db_init_cmds::db_init,
            db_init_cmds::db_get_vault_tree,
            vault_cmds::db_get_item,
            vault_cmds::db_update_item,
            vault_cmds::db_import_text,
            vault_cmds::db_archive_item,
            vault_cmds::db_delete_item,
            vault_cmds::db_create_vault_node,
            search_cmds::db_search_chunks,
            wards_cmds::wards_list,
            wards_cmds::wards_add,
            wards_cmds::wards_remove,
            wards_cmds::wards_scan,
            ai_cmds::ai_get_provider_settings,
            ai_cmds::ai_save_provider_settings,
            ai_cmds::ai_set_api_key,
            ai_cmds::ai_delete_api_key,
            ai_cmds::ai_accept_cloud_disclosure,
            ai_cmds::ai_select_provider,
            ai_cmds::ai_list_models,
            ai_cmds::ai_chat,
            ai_cmds::chat_with_vault,
            ollama_cmds::ollama_get_status,
            ollama_cmds::ollama_select_model,
            ollama_cmds::ollama_chat,
            export_cmds::export_item_markdown,
            export_cmds::export_vault_items_json,
            export_cmds::manuscript_export,
            export_cmds::reorder_item,
            export_cmds::export_project_json,
            external_vault_parse,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Grimoire");
}

#[tauri::command]
fn external_vault_parse(path: Option<String>) -> Result<external_vault::ExternalVaultStructure, String> {
    external_vault::parse_external_vault(path)
}

#[cfg(test)]
mod tests {
    #[test]
    fn external_vault_parse_handles_none_path() {
        let result = crate::external_vault::parse_external_vault(None);
        assert!(result.is_err());
    }
}
