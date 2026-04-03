use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub category: String,
    pub stack_size: u32,
    pub sink_value: u32,
    pub is_raw: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Machine {
    pub id: String,
    pub name: String,
    pub category: String,
    pub power_mw: f64,
    pub power_variable: bool,
    pub input_slots: u32,
    pub output_slots: u32,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeIngredient {
    pub item: String,
    pub amount: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    pub is_alternate: bool,
    pub machine: String,
    pub cycle_time_s: f64,
    pub inputs: Vec<RecipeIngredient>,
    pub outputs: Vec<RecipeIngredient>,
    pub unlock_tier: u32,
    pub notes: String,
}

impl Recipe {
    /// Items per minute for the given output item at 100% clock on one machine.
    pub fn output_rate(&self, item_id: &str) -> f64 {
        self.outputs
            .iter()
            .find(|o| o.item == item_id)
            .map(|o| (o.amount as f64 / self.cycle_time_s) * 60.0)
            .unwrap_or(0.0)
    }

    /// Items per minute for the given input item at 100% clock on one machine.
    pub fn input_rate(&self, item_id: &str) -> f64 {
        self.inputs
            .iter()
            .find(|i| i.item == item_id)
            .map(|i| (i.amount as f64 / self.cycle_time_s) * 60.0)
            .unwrap_or(0.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceNode {
    pub item: String,
    pub purity: String,
    pub node_count: u32,
    pub max_rate_per_node: f64,
}

/// A group of machines in a player's factory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryMachineGroup {
    pub id: String,
    pub machine: String,
    pub recipe: String,
    pub count: u32,
    pub clock_speed: f64,
    pub notes: String,
}

/// An I/O connection on a factory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryIO {
    pub item: String,
    pub rate: f64,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub destination: String,
}

/// A factory definition from the world state file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Factory {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub location: String,
    pub active: bool,
    pub machines: Vec<FactoryMachineGroup>,
    pub inputs: Vec<FactoryIO>,
    pub outputs: Vec<FactoryIO>,
    #[serde(default)]
    pub notes: String,
}

// --- Logistics ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildCost {
    pub item: String,
    pub amount: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConveyorBelt {
    pub id: String,
    pub name: String,
    pub tier: u32,
    pub rate_per_min: u32,
    pub unlock_tier: u32,
    pub unlock_milestone: String,
    pub build_cost: Vec<BuildCost>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub name: String,
    pub tier: u32,
    pub rate_per_min: u32,
    pub unlock_tier: u32,
    pub unlock_milestone: String,
    pub build_cost: Vec<BuildCost>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticsData {
    pub conveyor_belts: Vec<ConveyorBelt>,
    pub pipelines: Vec<Pipeline>,
}

// --- Milestones ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceElevatorPhase {
    pub phase: u32,
    pub name: String,
    pub requirements: Vec<BuildCost>,
    pub unlocks: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubMilestone {
    pub id: String,
    pub name: String,
    pub cost: Vec<BuildCost>,
    pub unlocks_machines: Vec<String>,
    pub unlocks_recipes: Vec<String>,
    pub unlocks_other: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubTier {
    pub tier: u32,
    pub name: String,
    pub milestones: Vec<HubMilestone>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MamNode {
    pub id: String,
    pub name: String,
    pub cost: Vec<BuildCost>,
    pub unlocks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MamTree {
    pub id: String,
    pub name: String,
    pub nodes: Vec<MamNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestonesData {
    pub space_elevator_phases: Vec<SpaceElevatorPhase>,
    pub tiers: Vec<HubTier>,
    pub mam_trees: Vec<MamTree>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_recipe() -> Recipe {
        Recipe {
            id: "test_recipe".to_string(),
            name: "Test Recipe".to_string(),
            is_alternate: false,
            machine: "constructor".to_string(),
            cycle_time_s: 6.0,
            inputs: vec![RecipeIngredient {
                item: "iron_ingot".to_string(),
                amount: 3,
            }],
            outputs: vec![
                RecipeIngredient {
                    item: "iron_rod".to_string(),
                    amount: 2,
                },
                RecipeIngredient {
                    item: "scrap".to_string(),
                    amount: 1,
                },
            ],
            unlock_tier: 0,
            notes: String::new(),
        }
    }

    #[test]
    fn output_rate_primary_output() {
        // 2 iron_rod per 6s = 20/min
        assert!((make_recipe().output_rate("iron_rod") - 20.0).abs() < 0.001);
    }

    #[test]
    fn output_rate_secondary_output() {
        // 1 scrap per 6s = 10/min
        assert!((make_recipe().output_rate("scrap") - 10.0).abs() < 0.001);
    }

    #[test]
    fn output_rate_unknown_item_returns_zero() {
        assert_eq!(make_recipe().output_rate("nonexistent"), 0.0);
    }

    #[test]
    fn input_rate_known_item() {
        // 3 iron_ingot per 6s = 30/min
        assert!((make_recipe().input_rate("iron_ingot") - 30.0).abs() < 0.001);
    }

    #[test]
    fn input_rate_unknown_item_returns_zero() {
        assert_eq!(make_recipe().input_rate("nonexistent"), 0.0);
    }
}
