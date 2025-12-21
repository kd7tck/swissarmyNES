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
    #[serde(default)]
    pub attrs: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AudioNote {
    pub pitch: u8,
    pub row: u8,
    pub col: u8,
    pub duration: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AudioEnvelope {
    pub name: String,
    pub steps: Vec<(i8, u8)>, // Value, Duration
    pub loop_index: Option<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AudioTrack {
    pub name: String,
    pub notes: Vec<AudioNote>,
    #[serde(default)]
    #[serde(alias = "envelope")]
    pub channel: u8, // 0=Pulse1, 1=Pulse2, 2=Triangle, 3=Noise (Renamed from envelope)
    #[serde(default)]
    pub instrument: u8, // Envelope/Duty setting (e.g. $9F for Max Vol, 50% Duty)
    #[serde(default)]
    pub priority: u8,
    #[serde(default)]
    pub vol_env: Option<u8>,
    #[serde(default)]
    pub pitch_env: Option<u8>,
    #[serde(default)]
    pub arpeggio_env: Option<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DpcmSample {
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SoundEffect {
    pub name: String,
    pub channel: u8,
    #[serde(default)]
    pub priority: u8,
    #[serde(default)]
    pub speed: u8,
    #[serde(default)]
    pub vol_sequence: Vec<u8>,
    #[serde(default)]
    pub pitch_sequence: Vec<i8>,
    #[serde(default)]
    pub duty_sequence: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectAssets {
    pub chr_bank: Vec<u8>,
    pub palettes: Vec<Palette>,
    pub nametables: Vec<Nametable>,
    #[serde(default)]
    pub audio_tracks: Vec<AudioTrack>,
    #[serde(default)]
    pub envelopes: Vec<AudioEnvelope>,
    #[serde(default)]
    pub samples: Vec<DpcmSample>,
    #[serde(default)]
    pub sound_effects: Vec<SoundEffect>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    pub metadata: ProjectMetadata,
    pub source: String,
    pub assets: Option<ProjectAssets>,
}

pub const PROJECTS_DIR: &str = "projects";

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
    validate_project_name(name)?;

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
        audio_tracks: vec![],    // Start empty
        envelopes: vec![],       // Start empty
        samples: vec![],         // Start empty
        sound_effects: vec![],   // Start empty
    };
    let assets_json = serde_json::to_string_pretty(&default_assets).map_err(|e| e.to_string())?;
    fs::write(project_path.join("assets.json"), assets_json).map_err(|e| e.to_string())?;

    Ok(())
}

pub fn get_project(name: &str) -> Result<Project, String> {
    validate_project_name(name)?;

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
        // Handle migration from 'envelope' to 'channel' if needed?
        // Serde default will handle missing fields. 'envelope' in JSON will be ignored, 'channel' will be 0.
        // We might want a custom deserialize to map envelope -> channel if strictly preserving data.
        // But for this project, defaults are acceptable or we assume the frontend will resave.
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
    validate_project_name(name)?;

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

pub fn list_files(project_name: &str) -> Result<Vec<String>, String> {
    validate_project_name(project_name)?;

    let project_path = Path::new(PROJECTS_DIR).join(project_name);
    if !project_path.exists() {
        return Err("Project not found".to_string());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(project_path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().is_file() {
            if let Some(name) = entry.file_name().to_str() {
                // Filter out system files if desired, or include all
                // For now, let's exclude the JSON metadata files from the code list
                // logic: include everything that is not a directory.
                // But specifically for the editor, we mostly care about .swiss files.
                // But the user might want to delete assets.json? No, that's dangerous.
                if name != "project.json" && name != "assets.json" {
                    files.push(name.to_string());
                }
            }
        }
    }
    files.sort();
    Ok(files)
}

pub fn read_file(project_name: &str, file_name: &str) -> Result<String, String> {
    validate_project_name(project_name)?;
    validate_filename(file_name)?;

    let path = Path::new(PROJECTS_DIR).join(project_name).join(file_name);
    if !path.exists() {
        return Err("File not found".to_string());
    }
    fs::read_to_string(path).map_err(|e| e.to_string())
}

pub fn write_file(project_name: &str, file_name: &str, content: &str) -> Result<(), String> {
    validate_project_name(project_name)?;
    validate_filename(file_name)?;

    // Prevent overwriting system files via this API
    if file_name == "project.json" || file_name == "assets.json" {
        return Err("Cannot modify system files via this API".to_string());
    }

    let project_path = Path::new(PROJECTS_DIR).join(project_name);
    if !project_path.exists() {
        return Err("Project not found".to_string());
    }

    let path = project_path.join(file_name);
    fs::write(path, content).map_err(|e| e.to_string())
}

pub fn delete_file(project_name: &str, file_name: &str) -> Result<(), String> {
    validate_project_name(project_name)?;
    validate_filename(file_name)?;

    if file_name == "main.swiss" {
        return Err("Cannot delete main entry point".to_string());
    }
    if file_name == "project.json" || file_name == "assets.json" {
        return Err("Cannot delete system files".to_string());
    }

    let path = Path::new(PROJECTS_DIR).join(project_name).join(file_name);
    if !path.exists() {
        return Err("File not found".to_string());
    }
    fs::remove_file(path).map_err(|e| e.to_string())
}

fn validate_project_name(name: &str) -> Result<(), String> {
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err("Invalid project name".to_string());
    }
    Ok(())
}

fn validate_filename(name: &str) -> Result<(), String> {
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("Invalid filename: Paths not allowed".to_string());
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err("Invalid filename: Invalid characters".to_string());
    }
    Ok(())
}
