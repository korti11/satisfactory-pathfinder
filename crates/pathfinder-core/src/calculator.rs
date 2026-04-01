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
