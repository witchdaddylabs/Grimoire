use super::*;

#[tauri::command]
pub fn app_ping() -> &'static str {
    "Grimoire desktop scaffold awake"
}

#[tauri::command]
pub fn project_create(request: ProjectCreateRequest) -> CommandResult<ProjectMetadata> {
    let parent_dir = match request.parent_dir {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path),
        _ => super::default_projects_dir()?,
    };

    let project_dir = parent_dir.join(super::project_folder_name(&request.name));
    fs::create_dir_all(&project_dir).map_err(|error| {
        format!(
            "Could not create project folder at {}: {error}",
            project_dir.display()
        )
    })?;

    let metadata = super::load_or_create_metadata(&project_dir, &request.name)?;
    let metadata = super::initialise_database(&metadata, request.seed_demo_data.unwrap_or(false))?;
    Ok(metadata)
}

#[tauri::command]
pub fn project_open(project_path: String) -> CommandResult<ProjectMetadata> {
    let project_dir = super::validate_project_dir(PathBuf::from(project_path))?;
    let metadata = super::read_metadata(&project_dir)?;
    let metadata = super::initialise_database(&metadata, false)?;
    Ok(metadata)
}

#[tauri::command]
pub fn project_get_metadata(project_path: String) -> CommandResult<ProjectMetadata> {
    let project_dir = super::validate_project_dir(PathBuf::from(project_path))?;
    super::read_metadata(&project_dir)
}
