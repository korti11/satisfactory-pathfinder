use crate::models::Recipe;

/// Result of an overclock calculation (fixed machine count → required clock speed).
#[derive(Debug, Clone)]
pub struct OverclockResult {
    pub recipe_id: String,
    pub recipe_name: String,
    pub machine_id: String,
    pub machine_count: u64,
    pub target_rate: f64,
    /// Required clock speed (1.0 = 100%). May exceed 2.5 if infeasible.
    pub clock_speed: f64,
    pub power_per_machine_mw: f64,
    pub total_power_mw: f64,
    /// Power Shards needed per machine (0–3). Capped at 3; see `feasible`.
    pub shards_per_machine: u32,
    pub total_shards: u32,
    /// False when the required clock exceeds 250% (2.5). Use `machines_at_max` instead.
    pub feasible: bool,
    /// Machines needed to hit the target at 250% clock (only meaningful when !feasible).
    pub machines_at_max_clock: u64,
}

/// Given a fixed number of machines, find the clock speed required to hit `target_rate`.
pub fn overclock(
    recipe: &Recipe,
    output_item_id: &str,
    machine_count: u64,
    target_rate: f64,
    base_machine_power_mw: f64,
) -> OverclockResult {
    let base_rate = recipe.output_rate(output_item_id);
    let clock = if base_rate > 0.0 && machine_count > 0 {
        target_rate / (base_rate * machine_count as f64)
    } else {
        0.0
    };

    let feasible = clock > 0.0 && clock <= 2.5;
    let clamped_clock = clock.min(2.5);

    let shards_per_machine: u32 = if clock > 1.0 {
        ((clock - 1.0) / 0.5).ceil() as u32
    } else {
        0
    }
    .min(3);

    let power_per_machine = power_at_clock(base_machine_power_mw, clamped_clock);
    let machines_at_max = if !feasible {
        ((target_rate / (base_rate * 2.5)).ceil() as u64).max(1)
    } else {
        machine_count
    };

    OverclockResult {
        recipe_id: recipe.id.clone(),
        recipe_name: recipe.name.clone(),
        machine_id: recipe.machine.clone(),
        machine_count,
        target_rate,
        clock_speed: clock,
        power_per_machine_mw: power_per_machine,
        total_power_mw: power_per_machine * machine_count as f64,
        shards_per_machine,
        total_shards: shards_per_machine * machine_count as u32,
        feasible,
        machines_at_max_clock: machines_at_max,
    }
}

/// Result of a single-machine rate calculation.
#[derive(Debug, Clone)]
pub struct CalcResult {
    pub recipe_id: String,
    pub recipe_name: String,
    pub machine_id: String,
    pub target_rate: f64,
    pub clock_speed: f64,
    /// Exact (fractional) number of machines needed.
    pub machines_exact: f64,
    pub inputs: Vec<RateEntry>,
    pub outputs: Vec<RateEntry>,
    pub power_mw: f64,
}

#[derive(Debug, Clone)]
pub struct RateEntry {
    pub item: String,
    pub rate: f64,
}

/// Power scaling formula: base_mw * (clock_speed ^ 1.321928)
pub fn power_at_clock(base_mw: f64, clock_speed: f64) -> f64 {
    base_mw * clock_speed.powf(1.321_928)
}

/// Calculate rates for producing `target_rate` items/min of `output_item_id`
/// using `recipe` at `clock_speed` (1.0 = 100%).
pub fn calculate(
    recipe: &Recipe,
    output_item_id: &str,
    target_rate: f64,
    clock_speed: f64,
    base_machine_power_mw: f64,
) -> CalcResult {
    let base_output_rate = recipe.output_rate(output_item_id);
    let effective_output_rate = base_output_rate * clock_speed;
    let machines_exact = if effective_output_rate > 0.0 {
        target_rate / effective_output_rate
    } else {
        0.0
    };

    let scale = machines_exact * clock_speed;

    let inputs = recipe
        .inputs
        .iter()
        .map(|i| RateEntry {
            item: i.item.clone(),
            rate: (i.amount as f64 / recipe.cycle_time_s) * 60.0 * scale,
        })
        .collect();

    let outputs = recipe
        .outputs
        .iter()
        .map(|o| RateEntry {
            item: o.item.clone(),
            rate: (o.amount as f64 / recipe.cycle_time_s) * 60.0 * scale,
        })
        .collect();

    let power_mw = power_at_clock(base_machine_power_mw, clock_speed) * machines_exact;

    CalcResult {
        recipe_id: recipe.id.clone(),
        recipe_name: recipe.name.clone(),
        machine_id: recipe.machine.clone(),
        target_rate,
        clock_speed,
        machines_exact,
        inputs,
        outputs,
        power_mw,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Recipe, RecipeIngredient};

    /// 1 iron_ingot → 1 iron_rod every 4s in a Constructor (4 MW).
    /// Base output rate = (1/4) * 60 = 15/min at 100% clock.
    fn make_recipe() -> Recipe {
        Recipe {
            id: "iron_rod_default".to_string(),
            name: "Iron Rod".to_string(),
            is_alternate: false,
            machine: "constructor".to_string(),
            cycle_time_s: 4.0,
            inputs: vec![RecipeIngredient { item: "iron_ingot".to_string(), amount: 1 }],
            outputs: vec![RecipeIngredient { item: "iron_rod".to_string(), amount: 1 }],
            unlock_tier: 0,
            notes: String::new(),
        }
    }

    // --- power_at_clock ---

    #[test]
    fn power_at_100_percent_equals_base() {
        assert!((power_at_clock(4.0, 1.0) - 4.0).abs() < 0.001);
    }

    #[test]
    fn power_scales_superlinearly_above_100() {
        // At 200% clock, power should be more than 2× base (super-linear)
        assert!(power_at_clock(4.0, 2.0) > 8.0);
    }

    #[test]
    fn power_scales_sublinearly_below_100() {
        // At 50% clock, power should be less than 0.5× base (sub-linear)
        assert!(power_at_clock(4.0, 0.5) < 2.0);
    }

    // --- calculate ---

    #[test]
    fn calculate_one_machine_at_base_rate() {
        // Targeting exactly 15/min requires exactly 1 machine at 100%
        let result = calculate(&make_recipe(), "iron_rod", 15.0, 1.0, 4.0);
        assert!((result.machines_exact - 1.0).abs() < 0.001);
        assert!((result.clock_speed - 1.0).abs() < 0.001);
    }

    #[test]
    fn calculate_multiple_machines() {
        // 45/min at 100% clock = 3 machines
        let result = calculate(&make_recipe(), "iron_rod", 45.0, 1.0, 4.0);
        assert!((result.machines_exact - 3.0).abs() < 0.001);
    }

    #[test]
    fn calculate_fractional_machines() {
        // 22.5/min at 100% clock = 1.5 machines
        let result = calculate(&make_recipe(), "iron_rod", 22.5, 1.0, 4.0);
        assert!((result.machines_exact - 1.5).abs() < 0.001);
    }

    #[test]
    fn calculate_input_rate_scales_with_output() {
        // 30/min iron_rod requires 30/min iron_ingot (1:1 recipe)
        let result = calculate(&make_recipe(), "iron_rod", 30.0, 1.0, 4.0);
        let ingot_input = result.inputs.iter().find(|i| i.item == "iron_ingot").unwrap();
        assert!((ingot_input.rate - 30.0).abs() < 0.001);
    }

    #[test]
    fn calculate_power_scales_with_machine_count() {
        // 2 machines at 100% clock = 2 × 4 MW = 8 MW
        let result = calculate(&make_recipe(), "iron_rod", 30.0, 1.0, 4.0);
        assert!((result.power_mw - 8.0).abs() < 0.01);
    }

    // --- overclock ---

    #[test]
    fn overclock_exactly_100_percent_needs_no_shards() {
        // 3 machines × 15/min = 45/min → clock = 1.0, 0 shards
        let result = overclock(&make_recipe(), "iron_rod", 3, 45.0, 4.0);
        assert!(result.feasible);
        assert!((result.clock_speed - 1.0).abs() < 0.001);
        assert_eq!(result.shards_per_machine, 0);
        assert_eq!(result.total_shards, 0);
    }

    #[test]
    fn overclock_150_percent_needs_one_shard() {
        // 1 machine, target 22.5/min → clock = 1.5 → 1 shard
        let result = overclock(&make_recipe(), "iron_rod", 1, 22.5, 4.0);
        assert!(result.feasible);
        assert!((result.clock_speed - 1.5).abs() < 0.001);
        assert_eq!(result.shards_per_machine, 1);
    }

    #[test]
    fn overclock_200_percent_needs_two_shards() {
        // 1 machine, target 30/min → clock = 2.0 → 2 shards
        let result = overclock(&make_recipe(), "iron_rod", 1, 30.0, 4.0);
        assert!(result.feasible);
        assert!((result.clock_speed - 2.0).abs() < 0.001);
        assert_eq!(result.shards_per_machine, 2);
    }

    #[test]
    fn overclock_250_percent_needs_three_shards() {
        // 1 machine, target 37.5/min → clock = 2.5 → 3 shards (max)
        let result = overclock(&make_recipe(), "iron_rod", 1, 37.5, 4.0);
        assert!(result.feasible);
        assert!((result.clock_speed - 2.5).abs() < 0.001);
        assert_eq!(result.shards_per_machine, 3);
    }

    #[test]
    fn overclock_infeasible_above_max_clock() {
        // 1 machine, target 60/min → clock = 4.0 → infeasible
        let result = overclock(&make_recipe(), "iron_rod", 1, 60.0, 4.0);
        assert!(!result.feasible);
        assert!(result.clock_speed > 2.5);
    }

    #[test]
    fn overclock_infeasible_gives_correct_machine_count_at_max() {
        // 1 machine infeasible at 60/min; at 250% = 37.5/min → need ceil(60/37.5) = 2
        let result = overclock(&make_recipe(), "iron_rod", 1, 60.0, 4.0);
        assert_eq!(result.machines_at_max_clock, 2);
    }

    #[test]
    fn overclock_total_shards_is_per_machine_times_count() {
        // 4 machines at 150% → 1 shard each → 4 total
        let result = overclock(&make_recipe(), "iron_rod", 4, 90.0, 4.0);
        assert_eq!(result.shards_per_machine, 1);
        assert_eq!(result.total_shards, 4);
    }
}
