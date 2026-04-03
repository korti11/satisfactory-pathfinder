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

fn pathfinder_embedded() -> Command {
    Command::cargo_bin("pathfinder").unwrap()
}

// ---------------------------------------------------------------------------
// embedded data
// ---------------------------------------------------------------------------

#[test]
fn embedded_data_loads_without_data_dir_flag() {
    pathfinder_embedded()
        .args(["list", "items", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("["));
}

// ---------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------

#[test]
fn list_resources_returns_json_array() {
    let output = pathfinder()
        .args(["list", "resources", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let resources: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert!(!resources.is_empty());
    assert!(resources
        .iter()
        .all(|r| r["max_rate_per_node"].as_f64().unwrap() > 0.0));
}

#[test]
fn list_resources_item_filter_returns_only_that_item() {
    let output = pathfinder()
        .args(["list", "resources", "--item", "iron_ore", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let resources: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert!(!resources.is_empty());
    assert!(resources
        .iter()
        .all(|r| r["item"].as_str().unwrap() == "iron_ore"));
}

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
        .args([
            "overclock",
            "Iron Rod",
            "--machines",
            "1",
            "--rate",
            "15",
            "--json",
        ])
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
        .args([
            "overclock",
            "Iron Rod",
            "--machines",
            "1",
            "--rate",
            "1000",
            "--json",
        ])
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
    assert!(items
        .iter()
        .all(|i| i["sink_value"].as_u64().unwrap_or(0) > 0));
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
    assert!(u["waste_per_min"].as_f64().unwrap() > p["waste_per_min"].as_f64().unwrap());
}

// ---------------------------------------------------------------------------
// list belts
// ---------------------------------------------------------------------------

#[test]
fn list_belts_returns_json_array() {
    let output = pathfinder()
        .args(["list", "belts", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let belts: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert!(!belts.is_empty());
    assert!(belts
        .iter()
        .all(|b| b["rate_per_min"].as_u64().unwrap() > 0));
}

#[test]
fn list_belts_are_ordered_by_tier() {
    let output = pathfinder()
        .args(["list", "belts", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let belts: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    let tiers: Vec<u64> = belts.iter().map(|b| b["tier"].as_u64().unwrap()).collect();
    assert!(tiers.windows(2).all(|w| w[0] <= w[1]));
}

// ---------------------------------------------------------------------------
// list pipes
// ---------------------------------------------------------------------------

#[test]
fn list_pipes_returns_json_array() {
    let output = pathfinder()
        .args(["list", "pipes", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let pipes: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert!(!pipes.is_empty());
    assert!(pipes
        .iter()
        .all(|p| p["rate_per_min"].as_u64().unwrap() > 0));
}

// ---------------------------------------------------------------------------
// list milestones
// ---------------------------------------------------------------------------

#[test]
fn list_milestones_returns_all_tiers() {
    let output = pathfinder()
        .args(["list", "milestones", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let tiers: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert!(!tiers.is_empty());
}

#[test]
fn list_milestones_tier_filter_returns_single_tier() {
    let output = pathfinder()
        .args(["list", "milestones", "--tier", "0", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let tiers: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert_eq!(tiers.len(), 1);
    assert_eq!(tiers[0]["tier"].as_u64().unwrap(), 0);
}

// ---------------------------------------------------------------------------
// list space-elevator
// ---------------------------------------------------------------------------

#[test]
fn list_space_elevator_returns_five_phases() {
    let output = pathfinder()
        .args(["list", "space-elevator", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let phases: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert_eq!(phases.len(), 5);
}

// ---------------------------------------------------------------------------
// list mam
// ---------------------------------------------------------------------------

#[test]
fn list_mam_returns_all_trees() {
    let output = pathfinder()
        .args(["list", "mam", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let trees: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert!(!trees.is_empty());
}

#[test]
fn list_mam_tree_filter_returns_single_tree() {
    let output = pathfinder()
        .args(["list", "mam", "--tree", "caterium", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let trees: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert_eq!(trees.len(), 1);
    assert_eq!(trees[0]["id"].as_str().unwrap(), "caterium");
}

#[test]
fn list_mam_unknown_tree_returns_empty_array() {
    let output = pathfinder()
        .args(["list", "mam", "--tree", "nonexistent", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let trees: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert!(trees.is_empty());
}

// ---------------------------------------------------------------------------
// companion install
// ---------------------------------------------------------------------------

#[test]
fn companion_install_project_creates_agent_file() {
    let tmp = std::env::temp_dir().join(format!("pathfinder_test_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();

    pathfinder()
        .args(["companion", "install"])
        .current_dir(&tmp)
        .assert()
        .success()
        .stdout(predicate::str::contains("satisfactory-companion.md"));

    let agent_path = tmp
        .join(".claude")
        .join("agents")
        .join("satisfactory-companion.md");
    assert!(
        agent_path.exists(),
        "agent file should exist at {}",
        agent_path.display()
    );

    let content = std::fs::read_to_string(&agent_path).unwrap();
    assert!(
        content.contains("satisfactory-companion"),
        "agent file should contain valid content"
    );

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn companion_install_global_creates_agent_file() {
    let tmp = std::env::temp_dir().join(format!("pathfinder_test_global_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();

    pathfinder()
        .args(["companion", "install", "--global"])
        .env("HOME", &tmp)
        .env("USERPROFILE", &tmp)
        .assert()
        .success()
        .stdout(predicate::str::contains("satisfactory-companion.md"));

    let agent_path = tmp
        .join(".claude")
        .join("agents")
        .join("satisfactory-companion.md");
    assert!(
        agent_path.exists(),
        "agent file should exist at {}",
        agent_path.display()
    );

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn companion_install_json_reports_install_path() {
    let tmp = std::env::temp_dir().join(format!("pathfinder_test_json_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();

    let output = pathfinder()
        .args(["companion", "install", "--json"])
        .current_dir(&tmp)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(result["installed"]
        .as_str()
        .unwrap()
        .contains("satisfactory-companion.md"));

    std::fs::remove_dir_all(&tmp).unwrap();
}

// ---------------------------------------------------------------------------
// progress show
// ---------------------------------------------------------------------------

#[test]
fn progress_show_returns_empty_state_when_no_file_exists() {
    let tmp = std::env::temp_dir().join(format!("pathfinder_progress_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();

    let output = pathfinder()
        .args([
            "--progress-file",
            tmp.join("progress.json").to_str().unwrap(),
            "progress",
            "show",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(result["milestones"].as_array().unwrap().len(), 0);
    assert_eq!(result["mam_nodes"].as_array().unwrap().len(), 0);
    assert_eq!(result["space_elevator_phases"].as_array().unwrap().len(), 0);
    assert_eq!(result["alternate_recipes"].as_array().unwrap().len(), 0);

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn progress_show_reads_existing_file() {
    let tmp = std::env::temp_dir().join(format!("pathfinder_progress_read_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let progress_file = tmp.join("progress.json");

    std::fs::write(
        &progress_file,
        r#"{"milestones":["basic_steel_production"],"mam_nodes":[],"space_elevator_phases":[1],"alternate_recipes":["alt_cast_screw"]}"#,
    )
    .unwrap();

    let output = pathfinder()
        .args([
            "--progress-file",
            progress_file.to_str().unwrap(),
            "progress",
            "show",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(result["milestones"].as_array().unwrap().len(), 1);
    assert_eq!(
        result["milestones"][0].as_str().unwrap(),
        "basic_steel_production"
    );
    assert_eq!(result["space_elevator_phases"][0].as_u64().unwrap(), 1);
    assert_eq!(
        result["alternate_recipes"][0].as_str().unwrap(),
        "alt_cast_screw"
    );

    std::fs::remove_dir_all(&tmp).unwrap();
}

// ---------------------------------------------------------------------------
// progress unlock/lock milestone
// ---------------------------------------------------------------------------

fn temp_progress_file(suffix: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let tmp = std::env::temp_dir().join(format!(
        "pathfinder_milestone_{}_{}",
        suffix,
        std::process::id()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let file = tmp.join("progress.json");
    (tmp, file)
}

#[test]
fn progress_unlock_milestone_adds_entry() {
    let (tmp, file) = temp_progress_file("unlock");

    let result = pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "unlock",
            "milestone",
            "basic_steel_production",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(json["milestone"].as_str().unwrap(), "basic_steel_production");
    assert_eq!(json["status"].as_str().unwrap(), "unlocked");

    // verify file was written
    let state: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert!(state["milestones"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m.as_str() == Some("basic_steel_production")));

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn progress_unlock_milestone_is_idempotent() {
    let (tmp, file) = temp_progress_file("idempotent");

    let args = [
        "--progress-file",
        file.to_str().unwrap(),
        "progress",
        "unlock",
        "milestone",
        "basic_steel_production",
        "--json",
    ];
    pathfinder().args(args).assert().success();

    let result = pathfinder()
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(json["status"].as_str().unwrap(), "already_unlocked");

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn progress_lock_milestone_removes_entry() {
    let (tmp, file) = temp_progress_file("lock");

    // unlock first
    pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "unlock",
            "milestone",
            "basic_steel_production",
        ])
        .assert()
        .success();

    // then lock
    let result = pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "lock",
            "milestone",
            "basic_steel_production",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(json["status"].as_str().unwrap(), "locked");

    let state: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert!(state["milestones"].as_array().unwrap().is_empty());

    std::fs::remove_dir_all(&tmp).unwrap();
}
