use std::collections::{HashMap, HashSet};

use crate::calculator::{calculate, CalcResult};
use crate::db::Db;

/// A node in the production chain tree.
#[derive(Debug, Clone)]
pub struct ChainNode {
    pub item_id: String,
    pub rate: f64,
    pub calc: Option<CalcResult>,
    pub children: Vec<ChainNode>,
    /// True when this item is treated as externally supplied (--assume flag).
    pub assumed: bool,
}

#[derive(Default)]
pub struct ChainOptions {
    /// Item ids that are already externally supplied (treated as raw inputs).
    pub assumed_supplies: HashMap<String, f64>,
    /// If true, skip alternate recipes.
    pub no_alternates: bool,
}

/// Recursively resolve the full production chain for `item_id` at `target_rate`.
pub fn resolve_chain(
    db: &Db,
    item_id: &str,
    target_rate: f64,
    opts: &ChainOptions,
    visited: &mut HashSet<String>,
) -> ChainNode {
    // Check if externally assumed
    if opts.assumed_supplies.contains_key(item_id) {
        return ChainNode {
            item_id: item_id.to_string(),
            rate: target_rate,
            calc: None,
            children: vec![],
            assumed: true,
        };
    }

    // Detect cycles (e.g. recipes that consume their own output)
    if visited.contains(item_id) {
        return ChainNode {
            item_id: item_id.to_string(),
            rate: target_rate,
            calc: None,
            children: vec![],
            assumed: false,
        };
    }

    // Raw resources are always leaf nodes — never try to produce them via a recipe
    if db.item(item_id).map(|i| i.is_raw).unwrap_or(false) {
        return ChainNode {
            item_id: item_id.to_string(),
            rate: target_rate,
            calc: None,
            children: vec![],
            assumed: false,
        };
    }

    // Find the default recipe for this item
    let recipe = if opts.no_alternates {
        db.recipes_for_item(item_id)
            .into_iter()
            .find(|r| !r.is_alternate && r.cycle_time_s > 0.0)
    } else {
        db.default_recipe_for_item(item_id)
            .filter(|r| r.cycle_time_s > 0.0)
    };

    let recipe = match recipe {
        Some(r) => r,
        None => {
            // Raw resource or unknown — leaf node
            return ChainNode {
                item_id: item_id.to_string(),
                rate: target_rate,
                calc: None,
                children: vec![],
                assumed: false,
            };
        }
    };

    let machine = db.machine(&recipe.machine);
    let base_power = machine.map(|m| m.power_mw).unwrap_or(0.0);
    let calc = calculate(recipe, item_id, target_rate, 1.0, base_power);

    visited.insert(item_id.to_string());

    let children = calc
        .inputs
        .iter()
        .map(|inp| resolve_chain(db, &inp.item, inp.rate, opts, visited))
        .collect();

    visited.remove(item_id);

    ChainNode {
        item_id: item_id.to_string(),
        rate: target_rate,
        calc: Some(calc),
        children,
        assumed: false,
    }
}

/// Collect a flat summary of total rates per raw resource.
pub fn raw_resource_summary(node: &ChainNode) -> HashMap<String, f64> {
    let mut totals: HashMap<String, f64> = HashMap::new();
    collect_raws(node, &mut totals);
    totals
}

fn collect_raws(node: &ChainNode, totals: &mut HashMap<String, f64>) {
    if node.calc.is_none() || node.assumed {
        *totals.entry(node.item_id.clone()).or_insert(0.0) += node.rate;
    } else {
        for child in &node.children {
            collect_raws(child, totals);
        }
    }
}
