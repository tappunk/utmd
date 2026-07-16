use chrono::{DateTime, Utc};
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmMetadata {
    pub name: String,
    pub os: Option<String>,
    pub template: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StateFile {
    pub vms: HashMap<String, VmMetadata>,
}

pub fn load(path: &Path) -> Result<StateFile> {
    if !path.exists() {
        return Ok(StateFile::default());
    }

    let content = fs::read_to_string(path)?;
    if content.trim().is_empty() {
        return Ok(StateFile::default());
    }

    let parsed: StateFile = serde_json::from_str(&content).map_err(|e| {
        eyre::eyre!(
            "state file '{}' is corrupted (json parse error): {}",
            path.display(),
            e
        )
    })?;
    Ok(parsed)
}

pub fn save(path: &Path, state: &StateFile) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(state)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .truncate(true)
        .open(&tmp)?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn upsert_vm(path: &Path, metadata: VmMetadata) -> Result<()> {
    let mut state = load(path)?;
    state.vms.insert(metadata.name.clone(), metadata);
    save(path, &state)
}

pub fn remove_vm(path: &Path, name: &str) -> Result<()> {
    let mut state = load(path)?;
    state.vms.remove(name);
    save(path, &state)
}

pub fn remove_vm_from_state(state: &mut StateFile, name: &str) {
    state.vms.remove(name);
}
