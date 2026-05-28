use crate::models::{
    ExternalVaultDrawer, ExternalVaultRoom, ExternalVaultStructure, ExternalVaultWing,
};
use serde_yaml::Value;
use std::{fs, path::PathBuf};

type ExternalResult<T> = Result<T, String>;

pub fn parse_external_vault(path: Option<String>) -> ExternalResult<ExternalVaultStructure> {
    let source_path = match path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(explicit) => PathBuf::from(explicit),
        None => {
            return Err("Choose an external Vault .yaml or .yml file.".to_string());
        }
    };

    let raw = fs::read_to_string(&source_path).map_err(|error| {
        format!(
            "Could not read external vault YAML at {}: {error}",
            source_path.display()
        )
    })?;
    let value: Value = serde_yaml::from_str(&raw).map_err(|error| {
        format!(
            "Could not parse external vault YAML at {}: {error}",
            source_path.display()
        )
    })?;

    let wings_value = value
        .as_mapping()
        .and_then(|mapping| mapping.get(Value::String("wings".to_string())))
        .unwrap_or(&value);

    let wings = parse_wings(wings_value);
    let total_wings = wings.len();
    let total_rooms = wings.iter().map(|wing| wing.rooms.len()).sum();
    let total_drawers = wings
        .iter()
        .flat_map(|wing| wing.rooms.iter())
        .map(|room| room.drawers.len())
        .sum();

    Ok(ExternalVaultStructure {
        wings,
        total_wings,
        total_rooms,
        total_drawers,
        source_file: source_path.to_string_lossy().to_string(),
    })
}

fn parse_wings(value: &Value) -> Vec<ExternalVaultWing> {
    match value {
        Value::Sequence(items) => items
            .iter()
            .filter_map(|item| parse_named_wing(None, item))
            .collect(),
        Value::Mapping(mapping) => mapping
            .iter()
            .filter_map(|(key, item)| parse_named_wing(value_to_string(key), item))
            .collect(),
        _ => Vec::new(),
    }
}

fn parse_named_wing(fallback_name: Option<String>, value: &Value) -> Option<ExternalVaultWing> {
    let mapping = value.as_mapping()?;
    let name = string_field(mapping, "name").or(fallback_name)?;
    let path = string_field(mapping, "path");
    let rooms = mapping
        .get(Value::String("rooms".to_string()))
        .map(parse_rooms)
        .unwrap_or_default();
    Some(ExternalVaultWing { name, path, rooms })
}

fn parse_rooms(value: &Value) -> Vec<ExternalVaultRoom> {
    match value {
        Value::Sequence(items) => items
            .iter()
            .filter_map(|item| parse_named_room(None, item))
            .collect(),
        Value::Mapping(mapping) => mapping
            .iter()
            .filter_map(|(key, item)| parse_named_room(value_to_string(key), item))
            .collect(),
        _ => Vec::new(),
    }
}

fn parse_named_room(fallback_name: Option<String>, value: &Value) -> Option<ExternalVaultRoom> {
    let mapping = value.as_mapping()?;
    let name = string_field(mapping, "name").or(fallback_name)?;
    let keywords = string_list_field(mapping, "keywords");
    let entities = string_list_field(mapping, "entities");
    let drawers = mapping
        .get(Value::String("drawers".to_string()))
        .map(parse_drawers)
        .unwrap_or_default();
    Some(ExternalVaultRoom {
        name,
        keywords,
        entities,
        drawers,
    })
}

fn parse_drawers(value: &Value) -> Vec<ExternalVaultDrawer> {
    match value {
        Value::Sequence(items) => items
            .iter()
            .filter_map(|item| parse_named_drawer(None, item))
            .collect(),
        Value::Mapping(mapping) => mapping
            .iter()
            .filter_map(|(key, item)| parse_named_drawer(value_to_string(key), item))
            .collect(),
        _ => Vec::new(),
    }
}

fn parse_named_drawer(fallback_name: Option<String>, value: &Value) -> Option<ExternalVaultDrawer> {
    match value {
        Value::Mapping(mapping) => {
            let name = string_field(mapping, "name").or(fallback_name)?;
            Some(ExternalVaultDrawer {
                name,
                keywords: string_list_field(mapping, "keywords"),
                descriptions: string_list_field(mapping, "descriptions"),
                entities: string_list_field(mapping, "entities"),
            })
        }
        _ => fallback_name.map(|name| ExternalVaultDrawer {
            name,
            keywords: Vec::new(),
            descriptions: Vec::new(),
            entities: Vec::new(),
        }),
    }
}

fn string_field(mapping: &serde_yaml::Mapping, key: &str) -> Option<String> {
    mapping
        .get(Value::String(key.to_string()))
        .and_then(value_to_string)
}

fn string_list_field(mapping: &serde_yaml::Mapping, key: &str) -> Vec<String> {
    mapping
        .get(Value::String(key.to_string()))
        .map(value_to_string_list)
        .unwrap_or_default()
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.trim().to_string()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
    .filter(|text| !text.is_empty())
}

fn value_to_string_list(value: &Value) -> Vec<String> {
    match value {
        Value::Sequence(items) => items.iter().filter_map(value_to_string).collect(),
        Value::String(text) => vec![text.trim().to_string()],
        _ => Vec::new(),
    }
}
