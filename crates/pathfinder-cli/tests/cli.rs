use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

/// Path to the shared data directory, resolved relative to this crate's manifest.
fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("data")
}

fn pathfinder() -> Command {
    let mut cmd = Command::cargo_bin("pathfinder").unwrap();
    cmd.arg("--data-dir").arg(data_dir());
    cmd
}

// ---------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------

#[test]
fn list_items_returns_json_array() {
    pathfinder()
        .args(["list", "items", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("["));
}

#[test]
fn list_items_category_filter_returns_only_that_category() {
    let output = pathfinder()
        .args(["list", "items", "--category", "ingot", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let items: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert!(!items.is_empty());
    assert!(items.iter().all(|i| i["category"] == "ingot"));
}

#[test]
fn list_items_unknown_category_returns_empty_array() {
    let output = pathfinder()
        .args(["list", "items", "--category", "nonexistent", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let items: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert!(items.is_empty());
}

#[test]
fn list_recipes_for_item_returns_matching_recipes() {
    let output = pathfinder()
        .args(["list", "recipes", "--item", "iron_rod", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let recipes: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert!(!recipes.is_empty());
    assert!(recipes.iter().all(|r| {
        r["outputs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["item"] == "iron_rod")
    }));
}

#[test]
fn list_recipes_alternate_flag_returns_only_alternates() {
    let output = pathfinder()
        .args(["list", "recipes", "--alternate", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let recipes: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert!(!recipes.is_empty());
    assert!(recipes.iter().all(|r| r["is_alternate"] == true));
}

#[test]
fn list_machines_returns_json_array() {
    pathfinder()
        .args(["list", "machines", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("["));
}

// ---------------------------------------------------------------------------
// calc
// ---------------------------------------------------------------------------

#[test]
fn calc_returns_correct_machine_count() {
    let output = pathfinder()
        .args(["calc", "Iron Rod", "--rate", "15", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
    // 15/min is exactly the base rate → 1 machine
    let machines: f64 = result["machines"].as_f64().unwrap();
    assert!((machines - 1.0).abs() < 0.01);
}

#[test]
fn calc_unknown_recipe_exits_with_error() {
    pathfinder()
        .args(["calc", "Nonexistent Item", "--rate", "10", "--json"])
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// chain
// ---------------------------------------------------------------------------

#[test]
fn chain_iron_rod_has_iron_ingot_child() {
    let output = pathfinder()
        .args(["chain", "Iron Rod", "--rate", "15", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let tree: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let children = tree["children"].as_array().unwrap();
    assert!(children.iter().any(|c| c["item"] == "iron_ingot"));
}

#[test]
fn chain_raw_resource_has_no_children() {
    let output = pathfinder()
        .args(["chain", "Iron Ore", "--rate", "60", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let tree: serde_json::Value = serde_json::from_slice(&output).unwrap();
    // Raw resources are leaf nodes with no children
    assert!(tree["children"].as_array().unwrap().is_empty());
}

#[test]
fn chain_unknown_item_exits_with_error() {
    pathfinder()
        .args(["chain", "Nonexistent Item", "--rate", "10", "--json"])
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// overclock
// ---------------------------------------------------------------------------

#[test]
fn overclock_one_machine_at_base_rate_is_100_percent() {
    let output = pathfinder()
        .args(["overclock", "Iron Rod", "--machines", "1", "--rate", "15", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(result["feasible"], true);
    let clock: f64 = result["clock_speed"].as_f64().unwrap();
    assert!((clock - 1.0).abs() < 0.01);
    assert_eq!(result["shards_per_machine"], 0);
}

#[test]
fn overclock_infeasible_reports_machines_at_max() {
    let output = pathfinder()
        .args(["overclock", "Iron Rod", "--machines", "1", "--rate", "1000", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(result["feasible"], false);
    assert!(result["machines_at_max_clock"].as_u64().unwrap() > 1);
}

// ---------------------------------------------------------------------------
// sink
// ---------------------------------------------------------------------------

#[test]
fn sink_list_returns_only_sinkable_items() {
    let output = pathfinder()
        .args(["sink", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let items: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert!(!items.is_empty());
    assert!(items.iter().all(|i| i["sink_value"].as_u64().unwrap_or(0) > 0));
}

#[test]
fn sink_item_with_rate_returns_points_per_min() {
    let output = pathfinder()
        .args(["sink", "--item", "Iron Rod", "--rate", "15", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(result["points_per_min"].as_f64().unwrap() > 0.0);
}

// ---------------------------------------------------------------------------
// nuclear
// ---------------------------------------------------------------------------

#[test]
fn nuclear_uranium_reports_correct_power_output() {
    let output = pathfinder()
        .args(["nuclear", "--plants", "1", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
    // 1 plant = 2500 MW
    assert!((result["power_output_mw"].as_f64().unwrap() - 2500.0).abs() < 0.1);
}

#[test]
fn nuclear_scales_linearly_with_plant_count() {
    let output = pathfinder()
        .args(["nuclear", "--plants", "4", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!((result["power_output_mw"].as_f64().unwrap() - 10000.0).abs() < 0.1);
}

#[test]
fn nuclear_plutonium_produces_less_waste_than_uranium() {
    let uranium = pathfinder()
        .args(["nuclear", "--plants", "1", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let plutonium = pathfinder()
        .args(["nuclear", "--plants", "1", "--fuel", "plutonium", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let u: serde_json::Value = serde_json::from_slice(&uranium).unwrap();
    let p: serde_json::Value = serde_json::from_slice(&plutonium).unwrap();
    assert!(
        u["waste_per_min"].as_f64().unwrap() > p["waste_per_min"].as_f64().unwrap()
    );
}
