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
