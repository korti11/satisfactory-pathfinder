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
// search
// ---------------------------------------------------------------------------

#[test]
fn search_returns_items_and_recipes_by_default() {
    let output = pathfinder()
        .args(["search", "iron", "plate", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let results: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r["type"] == "item"));
    assert!(results.iter().any(|r| r["type"] == "recipe"));
}

#[test]
fn search_items_flag_excludes_recipes() {
    let output = pathfinder()
        .args(["search", "iron", "plate", "--items", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let results: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r["type"] == "item"));
}

#[test]
fn search_recipes_flag_excludes_items() {
    let output = pathfinder()
        .args(["search", "iron", "plate", "--recipes", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let results: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r["type"] == "recipe"));
}

#[test]
fn search_mam_flag_returns_mam_nodes() {
    let output = pathfinder()
        .args(["search", "caterium", "--mam", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let results: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r["type"] == "mam"));
    assert!(results.iter().any(|r| r["tree"] == "Caterium"));
}

#[test]
fn search_milestones_flag_returns_milestones() {
    let output = pathfinder()
        .args(["search", "steel", "--milestones", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let results: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r["type"] == "milestone"));
}

#[test]
fn search_no_results_exits_successfully() {
    pathfinder()
        .args(["search", "xyzzy_no_such_thing"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No results"));
}

#[test]
fn search_result_fields_are_present() {
    let output = pathfinder()
        .args(["search", "iron", "plate", "--recipes", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let results: Vec<serde_json::Value> = serde_json::from_slice(&output).unwrap();
    let recipe = results
        .iter()
        .find(|r| r["id"] == "iron_plate_default")
        .unwrap();
    assert_eq!(recipe["machine"].as_str().unwrap(), "constructor");
    assert_eq!(recipe["is_alternate"].as_bool().unwrap(), false);
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
    assert!(result["path"]
        .as_str()
        .unwrap()
        .contains("satisfactory-companion.md"));
    assert_eq!(result["status"].as_str().unwrap(), "installed");

    std::fs::remove_dir_all(&tmp).unwrap();
}

// ---------------------------------------------------------------------------
// companion status
// ---------------------------------------------------------------------------

#[test]
fn companion_status_json_not_installed() {
    let tmp = std::env::temp_dir().join(format!(
        "pathfinder_test_status_none_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&tmp).unwrap();

    let output = pathfinder()
        .args(["companion", "status", "--json"])
        .current_dir(&tmp)
        .env("HOME", &tmp)
        .env("USERPROFILE", &tmp)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        result["project"]["status"].as_str().unwrap(),
        "not_installed"
    );
    assert_eq!(
        result["global"]["status"].as_str().unwrap(),
        "not_installed"
    );

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn companion_status_json_shows_up_to_date_after_install() {
    let tmp = std::env::temp_dir().join(format!(
        "pathfinder_test_status_uptodate_{}",
        std::process::id()
    ));
    // Separate home dir so project and global paths don't collide.
    let home = std::env::temp_dir().join(format!(
        "pathfinder_test_status_home_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::create_dir_all(&home).unwrap();

    // Install to project (CWD = tmp)
    pathfinder()
        .args(["companion", "install"])
        .current_dir(&tmp)
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .assert()
        .success();

    // Check status
    let output = pathfinder()
        .args(["companion", "status", "--json"])
        .current_dir(&tmp)
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(result["project"]["status"].as_str().unwrap(), "up_to_date");
    assert_eq!(
        result["global"]["status"].as_str().unwrap(),
        "not_installed"
    );

    std::fs::remove_dir_all(&tmp).unwrap();
    std::fs::remove_dir_all(&home).unwrap();
}

#[test]
fn companion_status_human_readable_output() {
    let tmp = std::env::temp_dir().join(format!(
        "pathfinder_test_status_human_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&tmp).unwrap();

    pathfinder()
        .args(["companion", "status"])
        .current_dir(&tmp)
        .env("HOME", &tmp)
        .env("USERPROFILE", &tmp)
        .assert()
        .success()
        .stdout(predicate::str::contains("Project"))
        .stdout(predicate::str::contains("Global"))
        .stdout(predicate::str::contains("not installed"));

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn companion_install_twice_reports_already_up_to_date() {
    let tmp = std::env::temp_dir().join(format!(
        "pathfinder_test_install_idempotent_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&tmp).unwrap();

    // First install
    pathfinder()
        .args(["companion", "install", "--json"])
        .current_dir(&tmp)
        .assert()
        .success();

    // Second install
    let output = pathfinder()
        .args(["companion", "install", "--json"])
        .current_dir(&tmp)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(result["status"].as_str().unwrap(), "already_up_to_date");

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
        r#"{"milestones":["basic_steel_production"],"mam_nodes":[],"space_elevator_phases":[1],"alternate_recipes":["alt_pure_iron_ingot"]}"#,
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
        "alt_pure_iron_ingot"
    );

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn progress_show_locked_excludes_unlocked_milestones() {
    let (tmp, file) = temp_progress_file("show_locked");

    // Unlock one milestone
    pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "unlock",
            "milestone",
            "hub_upgrade_1",
        ])
        .assert()
        .success();

    // --locked should not include hub_upgrade_1
    pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "show",
            "--locked",
            "--milestones",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("HUB Upgrade 1").not())
        .stdout(predicate::str::contains("HUB Upgrade 2"));

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn progress_show_milestones_filter_shows_only_milestones() {
    let (tmp, file) = temp_progress_file("show_milestones");

    pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "unlock",
            "milestone",
            "hub_upgrade_1",
        ])
        .assert()
        .success();

    pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "show",
            "--milestones",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Milestones"))
        .stdout(predicate::str::contains("MAM nodes").not())
        .stdout(predicate::str::contains("Space Elevator").not())
        .stdout(predicate::str::contains("Alternate").not());

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
    assert_eq!(
        json["milestone"].as_str().unwrap(),
        "basic_steel_production"
    );
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

// ---------------------------------------------------------------------------
// progress unlock/lock mam
// ---------------------------------------------------------------------------

#[test]
fn progress_unlock_mam_adds_entry() {
    let (tmp, file) = temp_progress_file("mam_unlock");

    let result = pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "unlock",
            "mam",
            "caterium_ore_scan",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(json["mam_node"].as_str().unwrap(), "caterium_ore_scan");
    assert_eq!(json["status"].as_str().unwrap(), "unlocked");

    let state: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert!(state["mam_nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m.as_str() == Some("caterium_ore_scan")));

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn progress_unlock_mam_is_idempotent() {
    let (tmp, file) = temp_progress_file("mam_idempotent");

    let args = [
        "--progress-file",
        file.to_str().unwrap(),
        "progress",
        "unlock",
        "mam",
        "caterium_ore_scan",
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
fn progress_lock_mam_removes_entry() {
    let (tmp, file) = temp_progress_file("mam_lock");

    pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "unlock",
            "mam",
            "caterium_ore_scan",
        ])
        .assert()
        .success();

    let result = pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "lock",
            "mam",
            "caterium_ore_scan",
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
    assert!(state["mam_nodes"].as_array().unwrap().is_empty());

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn progress_unlock_milestone_rejects_unknown_id() {
    let (tmp, file) = temp_progress_file("milestone_unknown");

    pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "unlock",
            "milestone",
            "not_a_real_milestone",
        ])
        .assert()
        .failure();

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn progress_unlock_mam_rejects_unknown_id() {
    let (tmp, file) = temp_progress_file("mam_unknown");

    pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "unlock",
            "mam",
            "not_a_real_node",
        ])
        .assert()
        .failure();

    std::fs::remove_dir_all(&tmp).unwrap();
}

// ---------------------------------------------------------------------------
// progress unlock/lock phase
// ---------------------------------------------------------------------------

#[test]
fn progress_unlock_phase_adds_entry() {
    let (tmp, file) = temp_progress_file("phase_unlock");

    let result = pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "unlock",
            "phase",
            "1",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(json["phase"].as_u64().unwrap(), 1);
    assert_eq!(json["status"].as_str().unwrap(), "unlocked");

    let state: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert!(state["space_elevator_phases"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p.as_u64() == Some(1)));

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn progress_unlock_phase_is_idempotent() {
    let (tmp, file) = temp_progress_file("phase_idempotent");

    // Unlock phase 1 first (required prerequisite)
    pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "unlock",
            "phase",
            "1",
        ])
        .assert()
        .success();

    let args = [
        "--progress-file",
        file.to_str().unwrap(),
        "progress",
        "unlock",
        "phase",
        "1",
        "--json",
    ];

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
fn progress_unlock_phase_rejects_out_of_range() {
    let (tmp, file) = temp_progress_file("phase_range");

    pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "unlock",
            "phase",
            "6",
        ])
        .assert()
        .failure();

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn progress_unlock_phase_rejects_skipping_previous() {
    let (tmp, file) = temp_progress_file("phase_skip");

    // Try to unlock phase 2 without phase 1
    pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "unlock",
            "phase",
            "2",
        ])
        .assert()
        .failure();

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn progress_lock_phase_removes_entry() {
    let (tmp, file) = temp_progress_file("phase_lock");

    // Unlock phases 1, 2, 3 sequentially
    for phase in ["1", "2", "3"] {
        pathfinder()
            .args([
                "--progress-file",
                file.to_str().unwrap(),
                "progress",
                "unlock",
                "phase",
                phase,
            ])
            .assert()
            .success();
    }

    let result = pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "lock",
            "phase",
            "3",
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
    let phases: Vec<u64> = state["space_elevator_phases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_u64().unwrap())
        .collect();
    assert!(!phases.contains(&3), "phase 3 should have been removed");
    assert!(
        phases.contains(&1) && phases.contains(&2),
        "phases 1 and 2 should remain"
    );

    std::fs::remove_dir_all(&tmp).unwrap();
}

// ---------------------------------------------------------------------------
// progress unlock/lock alt
// ---------------------------------------------------------------------------

#[test]
fn progress_unlock_alt_adds_entry() {
    let (tmp, file) = temp_progress_file("alt_unlock");

    let result = pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "unlock",
            "alt",
            "alt_pure_iron_ingot",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(json["alt_recipe"].as_str().unwrap(), "alt_pure_iron_ingot");
    assert_eq!(json["status"].as_str().unwrap(), "unlocked");

    let state: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert!(state["alternate_recipes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r.as_str() == Some("alt_pure_iron_ingot")));

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn progress_unlock_alt_is_idempotent() {
    let (tmp, file) = temp_progress_file("alt_idempotent");

    let args = [
        "--progress-file",
        file.to_str().unwrap(),
        "progress",
        "unlock",
        "alt",
        "alt_pure_iron_ingot",
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
fn progress_unlock_alt_rejects_missing_prefix() {
    let (tmp, file) = temp_progress_file("alt_prefix");

    pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "unlock",
            "alt",
            "cast_screw",
        ])
        .assert()
        .failure();

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn progress_unlock_alt_rejects_unknown_id() {
    let (tmp, file) = temp_progress_file("alt_unknown");

    pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "unlock",
            "alt",
            "alt_not_a_real_recipe",
        ])
        .assert()
        .failure();

    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn progress_lock_alt_removes_entry() {
    let (tmp, file) = temp_progress_file("alt_lock");

    pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "unlock",
            "alt",
            "alt_pure_iron_ingot",
        ])
        .assert()
        .success();

    let result = pathfinder()
        .args([
            "--progress-file",
            file.to_str().unwrap(),
            "progress",
            "lock",
            "alt",
            "alt_pure_iron_ingot",
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
    assert!(state["alternate_recipes"].as_array().unwrap().is_empty());

    std::fs::remove_dir_all(&tmp).unwrap();
}
