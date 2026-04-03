use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const PROGRESS_FILENAME: &str = "progress.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProgressState {
    /// Individual milestone IDs that have been unlocked (e.g. "basic_steel_production")
    #[serde(default)]
    pub milestones: Vec<String>,
    /// MAM node IDs that have been researched (e.g. "caterium_research")
    #[serde(default)]
    pub mam_nodes: Vec<String>,
    /// Space Elevator phases that have been submitted (1–5)
    #[serde(default)]
    pub space_elevator_phases: Vec<u32>,
    /// Alternate recipe IDs found via Hard Drives (e.g. "alt_cast_screw")
    #[serde(default)]
    pub alternate_recipes: Vec<String>,
}

pub fn load(path: &Path) -> Result<ProgressState> {
    if !path.exists() {
        return Ok(ProgressState::default());
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn save(path: &Path, state: &ProgressState) -> Result<()> {
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(path, json).with_context(|| format!("failed to write {}", path.display()))
}
