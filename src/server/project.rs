use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectMetadata {
    pub name: String,
    pub created_at: u64, // Timestamp
    pub modified_at: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Palette {
    pub name: String,
    pub colors: [u8; 4],
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Nametable {
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectAssets {
    pub chr_bank: Vec<u8>,
    pub palettes: Vec<Palette>,
    pub nametables: Vec<Nametable>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    pub metadata: ProjectMetadata,
    pub source: String,
    pub assets: Option<ProjectAssets>,
}

const PROJECTS_DIR: &str = "projects";

pub fn list_projects() -> Result<Vec<String>, String> {
    let path = Path::new(PROJECTS_DIR);
    if !path.exists() {
        fs::create_dir(path).map_err(|e| e.to_string())?;
    }

    let mut projects = Vec::new();
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                projects.push(name.to_string());
            }
        }
    }
    // Sort projects for consistent output
    projects.sort();
    Ok(projects)
}

pub fn create_project(name: &str) -> Result<(), String> {
    // Validate name (simple alphanumeric check)
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err("Invalid project name".to_string());
    }

    let project_path = Path::new(PROJECTS_DIR).join(name);
    if project_path.exists() {
        return Err("Project already exists".to_string());
    }

    fs::create_dir_all(&project_path).map_err(|e| e.to_string())?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();

    let metadata = ProjectMetadata {
        name: name.to_string(),
        created_at: now,
        modified_at: now,
    };
    let meta_json = serde_json::to_string_pretty(&metadata).map_err(|e| e.to_string())?;
    fs::write(project_path.join("project.json"), meta_json).map_err(|e| e.to_string())?;

    // Default source
    let default_source = "CONST BG_COLOR = $0F\n\nSUB Main()\n    ' Set Palette Address $3F00\n    POKE($2006, $3F)\n    POKE($2006, $00)\n    ' Write Color\n    POKE($2007, BG_COLOR)\nEND SUB";
    fs::write(project_path.join("main.swiss"), default_source).map_err(|e| e.to_string())?;

    // Default assets
    let default_assets = ProjectAssets {
        chr_bank: vec![0; 4096], // 4KB empty CHR
        palettes: vec![],        // Start empty
        nametables: vec![],      // Start empty
    };
    let assets_json = serde_json::to_string_pretty(&default_assets).map_err(|e| e.to_string())?;
    fs::write(project_path.join("assets.json"), assets_json).map_err(|e| e.to_string())?;

    Ok(())
}

pub fn get_project(name: &str) -> Result<Project, String> {
    // Validate name
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err("Invalid project name".to_string());
    }

    let project_path = Path::new(PROJECTS_DIR).join(name);
    if !project_path.exists() {
        return Err("Project not found".to_string());
    }

    let meta_content =
        fs::read_to_string(project_path.join("project.json")).map_err(|e| e.to_string())?;
    let metadata: ProjectMetadata =
        serde_json::from_str(&meta_content).map_err(|e| e.to_string())?;

    let source = fs::read_to_string(project_path.join("main.swiss")).map_err(|e| e.to_string())?;

    let assets_path = project_path.join("assets.json");
    let assets = if assets_path.exists() {
        let assets_content = fs::read_to_string(assets_path).map_err(|e| e.to_string())?;
        Some(serde_json::from_str(&assets_content).map_err(|e| e.to_string())?)
    } else {
        None
    };

    Ok(Project {
        metadata,
        source,
        assets,
    })
}

pub fn save_project(
    name: &str,
    source: Option<&str>,
    assets: Option<&ProjectAssets>,
) -> Result<(), String> {
    // Validate name
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err("Invalid project name".to_string());
    }

    let project_path = Path::new(PROJECTS_DIR).join(name);
    if !project_path.exists() {
        return Err("Project not found".to_string());
    }

    if let Some(src) = source {
        fs::write(project_path.join("main.swiss"), src).map_err(|e| e.to_string())?;
    }

    if let Some(asset_data) = assets {
        let assets_json = serde_json::to_string_pretty(asset_data).map_err(|e| e.to_string())?;
        fs::write(project_path.join("assets.json"), assets_json).map_err(|e| e.to_string())?;
    }

    // Update modified time in metadata
    let meta_path = project_path.join("project.json");
    if let Ok(meta_content) = fs::read_to_string(&meta_path) {
        if let Ok(mut metadata) = serde_json::from_str::<ProjectMetadata>(&meta_content) {
            if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
                metadata.modified_at = now.as_secs();
                if let Ok(new_meta) = serde_json::to_string_pretty(&metadata) {
                    let _ = fs::write(meta_path, new_meta);
                }
            }
        }
    }

    Ok(())
}
