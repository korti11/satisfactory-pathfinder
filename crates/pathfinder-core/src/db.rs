use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use rust_embed::Embed;

use crate::models::{Factory, Item, LogisticsData, Machine, MilestonesData, Recipe, ResourceNode};

#[derive(Embed)]
#[folder = "../../data/"]
struct EmbeddedData;

pub struct Db {
    items: HashMap<String, Item>,
    machines: HashMap<String, Machine>,
    recipes: Vec<Recipe>,
    resources: Vec<ResourceNode>,
    logistics: LogisticsData,
    milestones: MilestonesData,
}

impl Db {
    /// Load game data from an optional directory override.
    /// Falls back to data embedded in the binary if no directory is given.
    pub fn load(data_dir: Option<&Path>) -> Result<Self> {
        let load = |filename: &str| -> Result<Vec<u8>> {
            match data_dir {
                Some(dir) => Ok(std::fs::read(dir.join(filename))
                    .with_context(|| format!("failed to read {filename}"))?),
                None => Ok(EmbeddedData::get(filename)
                    .with_context(|| format!("embedded data file '{filename}' not found"))?
                    .data
                    .to_vec()),
            }
        };

        let items: Vec<Item> = parse_json(&load("items.json")?, "items.json")?;
        let machines: Vec<Machine> = parse_json(&load("machines.json")?, "machines.json")?;
        let recipes: Vec<Recipe> = parse_json(&load("recipes.json")?, "recipes.json")?;
        let resources: Vec<ResourceNode> = parse_json(&load("resources.json")?, "resources.json")?;
        let logistics: LogisticsData = parse_json(&load("logistics.json")?, "logistics.json")?;
        let milestones: MilestonesData = parse_json(&load("milestones.json")?, "milestones.json")?;

        Ok(Self {
            items: items.into_iter().map(|i| (i.id.clone(), i)).collect(),
            machines: machines.into_iter().map(|m| (m.id.clone(), m)).collect(),
            recipes,
            resources,
            logistics,
            milestones,
        })
    }

    // --- Items ---

    pub fn item(&self, id: &str) -> Option<&Item> {
        self.items.get(id)
    }

    pub fn all_items(&self) -> impl Iterator<Item = &Item> {
        self.items.values()
    }

    pub fn items_by_category<'a>(&'a self, category: &'a str) -> impl Iterator<Item = &'a Item> {
        self.items.values().filter(move |i| i.category == category)
    }

    // --- Machines ---

    pub fn machine(&self, id: &str) -> Option<&Machine> {
        self.machines.get(id)
    }

    pub fn all_machines(&self) -> impl Iterator<Item = &Machine> {
        self.machines.values()
    }

    // --- Recipes ---

    pub fn recipe(&self, id: &str) -> Option<&Recipe> {
        self.recipes.iter().find(|r| r.id == id)
    }

    pub fn all_recipes(&self) -> impl Iterator<Item = &Recipe> {
        self.recipes.iter()
    }

    pub fn recipes_for_item(&self, item_id: &str) -> Vec<&Recipe> {
        self.recipes
            .iter()
            .filter(|r| r.outputs.iter().any(|o| o.item == item_id))
            .collect()
    }

    pub fn default_recipe_for_item(&self, item_id: &str) -> Option<&Recipe> {
        // Prefer the _default recipe, fall back to first non-alternate
        self.recipes_for_item(item_id)
            .into_iter()
            .find(|r| r.id == format!("{}_default", item_id))
            .or_else(|| {
                self.recipes_for_item(item_id)
                    .into_iter()
                    .find(|r| !r.is_alternate)
            })
    }

    /// Find a recipe by id or by the output item name/id (case-insensitive).
    pub fn find_recipe(&self, query: &str) -> Option<&Recipe> {
        let lower = query.to_lowercase();
        // Exact id match first
        if let Some(r) = self.recipe(&lower) {
            return Some(r);
        }
        // Try as item id → default recipe
        if let Some(r) = self.default_recipe_for_item(&lower) {
            return Some(r);
        }
        // Try matching item name (case-insensitive) → default recipe
        if let Some(item) = self.items.values().find(|i| i.name.to_lowercase() == lower) {
            if let Some(r) = self.default_recipe_for_item(&item.id) {
                return Some(r);
            }
        }
        // Try recipe name (case-insensitive)
        self.recipes.iter().find(|r| r.name.to_lowercase() == lower)
    }

    // --- Resources ---

    pub fn all_resources(&self) -> impl Iterator<Item = &ResourceNode> {
        self.resources.iter()
    }

    pub fn resources_for_item(&self, item_id: &str) -> Vec<&ResourceNode> {
        self.resources
            .iter()
            .filter(|r| r.item == item_id)
            .collect()
    }

    /// Total extraction capacity (items/min) for an item at 100% clock with Mk.2 miners.
    pub fn max_extraction_rate(&self, item_id: &str) -> f64 {
        self.resources_for_item(item_id)
            .iter()
            .map(|r| r.node_count as f64 * r.max_rate_per_node)
            .sum()
    }

    // --- Logistics ---

    pub fn conveyor_belts(&self) -> &[crate::models::ConveyorBelt] {
        &self.logistics.conveyor_belts
    }

    pub fn pipelines(&self) -> &[crate::models::Pipeline] {
        &self.logistics.pipelines
    }

    // --- Milestones ---

    pub fn space_elevator_phases(&self) -> &[crate::models::SpaceElevatorPhase] {
        &self.milestones.space_elevator_phases
    }

    pub fn hub_tiers(&self) -> &[crate::models::HubTier] {
        &self.milestones.tiers
    }

    pub fn mam_trees(&self) -> &[crate::models::MamTree] {
        &self.milestones.mam_trees
    }
}

/// Load and deserialize a JSON file from disk (used for factory files).
pub fn load_factories(path: &Path) -> Result<Vec<Factory>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))
}

fn parse_json<T: serde::de::DeserializeOwned>(data: &[u8], name: &str) -> Result<T> {
    let content =
        std::str::from_utf8(data).with_context(|| format!("'{name}' is not valid UTF-8"))?;
    serde_json::from_str(content).with_context(|| format!("failed to parse '{name}'"))
}
