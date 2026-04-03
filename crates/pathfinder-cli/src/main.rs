mod output;

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde_json::json;

use output::Formatter;
use pathfinder_core::bottleneck::analyse_factory;
use pathfinder_core::calculator::{calculate, overclock};
use pathfinder_core::chain::{resolve_chain, ChainNode, ChainOptions};
use pathfinder_core::db::{load_factories, Db};
use pathfinder_core::progress;

#[derive(Parser)]
#[command(
    name = "pathfinder",
    version = env!("APP_VERSION"),
    about = "Satisfactory factory planning companion"
)]
struct Cli {
    /// Path to a data directory to override the embedded game data
    #[arg(long)]
    data_dir: Option<PathBuf>,

    /// Path to the world progress file (default: ./progress.json)
    #[arg(long)]
    progress_file: Option<PathBuf>,

    /// Output machine-readable JSON (for programmatic use)
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Browse the game knowledge base
    List {
        #[command(subcommand)]
        target: ListTarget,
    },
    /// Calculate rates for a single machine running one recipe
    Calc {
        /// Recipe id or output item name/id
        recipe: String,
        /// Target output rate (items/min)
        #[arg(long)]
        rate: f64,
        /// Clock speed percentage (default: 100)
        #[arg(long, default_value = "100")]
        clock: f64,
    },
    /// Recursively resolve the full production chain for an item
    Chain {
        /// Output item name or id
        item: String,
        /// Target output rate (items/min)
        #[arg(long)]
        rate: f64,
        /// Skip alternate recipes
        #[arg(long)]
        no_alternates: bool,
        /// Treat item as externally supplied: --assume iron_ingot:240
        #[arg(long, value_name = "ITEM:RATE")]
        assume: Vec<String>,
    },
    /// Analyse a factory file for throughput problems
    Bottleneck {
        /// Path to a single factory JSON file
        #[arg(long)]
        factory: Option<PathBuf>,
        /// Path to a world JSON file containing multiple factories
        #[arg(long)]
        world: Option<PathBuf>,
    },
    /// Find the clock speed needed for N machines to hit a target rate
    Overclock {
        /// Recipe id or output item name/id
        recipe: String,
        /// Number of machines available
        #[arg(long)]
        machines: u64,
        /// Desired output rate (items/min)
        #[arg(long)]
        rate: f64,
    },
    /// List sinkable items ranked by AWESOME Sink point value
    Sink {
        /// Filter by item name or id (shows points/min at the given rate)
        #[arg(long)]
        item: Option<String>,
        /// Production rate for points/min calculation (requires --item)
        #[arg(long)]
        rate: Option<f64>,
        /// Filter by item category
        #[arg(long)]
        category: Option<String>,
    },
    /// Show nuclear power plant resource rates and waste output
    Nuclear {
        /// Number of nuclear power plants
        #[arg(long)]
        plants: u64,
        /// Fuel type: uranium (default) or plutonium
        #[arg(long, default_value = "uranium")]
        fuel: String,
    },
    /// Manage the Satisfactory companion agent
    Companion {
        #[command(subcommand)]
        action: CompanionAction,
    },
    /// Track world progress (milestones, MAM, Space Elevator, alternate recipes)
    Progress {
        #[command(subcommand)]
        action: ProgressAction,
    },
}

#[derive(Subcommand)]
enum ProgressAction {
    /// Show current world progress
    Show,
    /// Mark something as unlocked/completed
    Unlock {
        #[command(subcommand)]
        target: ProgressTarget,
    },
    /// Undo a previously recorded unlock
    Lock {
        #[command(subcommand)]
        target: ProgressTarget,
    },
}

#[derive(Subcommand, Clone)]
enum ProgressTarget {
    /// A HUB milestone (by id, e.g. basic_steel_production)
    Milestone {
        /// Milestone id
        id: String,
    },
    /// A MAM research node (by id, e.g. caterium_research)
    Mam {
        /// MAM node id
        id: String,
    },
    /// A Space Elevator phase (1–5)
    Phase {
        /// Phase number (1–5)
        number: u32,
    },
    /// A Hard Drive alternate recipe (by id, e.g. alt_cast_screw)
    Alt {
        /// Alternate recipe id (must start with alt_)
        id: String,
    },
}

#[derive(Subcommand)]
enum CompanionAction {
    /// Install the companion agent to .claude/agents/ (or ~/.claude/agents/ with --global)
    Install {
        /// Install to ~/.claude/agents/ instead of .claude/agents/ in the current directory
        #[arg(long)]
        global: bool,
    },
}

#[derive(Subcommand)]
enum ListTarget {
    /// List items
    Items {
        #[arg(long)]
        category: Option<String>,
    },
    /// List recipes
    Recipes {
        /// Show only recipes that produce this item id
        #[arg(long)]
        item: Option<String>,
        /// Show only alternate recipes
        #[arg(long)]
        alternate: bool,
    },
    /// List machines
    Machines,
    /// List resource nodes and their extraction rates
    Resources {
        /// Filter by item id (e.g. iron_ore)
        #[arg(long)]
        item: Option<String>,
    },
    /// List conveyor belt tiers and throughput rates
    Belts,
    /// List pipeline tiers and throughput rates
    Pipes,
    /// List HUB milestones, optionally filtered by tier
    Milestones {
        /// Show only milestones for this tier number
        #[arg(long)]
        tier: Option<u32>,
    },
    /// List Space Elevator phases and requirements
    SpaceElevator,
    /// List MAM research trees, optionally filtered by tree id
    Mam {
        /// Show only this research tree (e.g. caterium, quartz)
        #[arg(long)]
        tree: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let fmt = Formatter::new(cli.json);
    let db = Db::load(cli.data_dir.as_deref())?;
    let progress_path = cli
        .progress_file
        .unwrap_or_else(|| PathBuf::from(progress::PROGRESS_FILENAME));

    match cli.command {
        Commands::List { target } => cmd_list(&db, &fmt, target),
        Commands::Calc {
            recipe,
            rate,
            clock,
        } => cmd_calc(&db, &fmt, &recipe, rate, clock / 100.0),
        Commands::Chain {
            item,
            rate,
            no_alternates,
            assume,
        } => cmd_chain(&db, &fmt, &item, rate, no_alternates, &assume),
        Commands::Bottleneck { factory, world } => cmd_bottleneck(&db, &fmt, factory, world),
        Commands::Overclock {
            recipe,
            machines,
            rate,
        } => cmd_overclock(&db, &fmt, &recipe, machines, rate),
        Commands::Sink {
            item,
            rate,
            category,
        } => cmd_sink(&db, &fmt, item.as_deref(), rate, category.as_deref()),
        Commands::Nuclear { plants, fuel } => cmd_nuclear(&fmt, plants, &fuel),
        Commands::Companion { action } => cmd_companion(&fmt, action),
        Commands::Progress { action } => cmd_progress(&fmt, &progress_path, action),
    }
}

// ---------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------

fn cmd_list(db: &Db, fmt: &Formatter, target: ListTarget) -> Result<()> {
    match target {
        ListTarget::Items { category } => {
            let mut items: Vec<_> = if let Some(cat) = &category {
                db.items_by_category(cat).collect()
            } else {
                db.all_items().collect()
            };
            items.sort_by(|a, b| a.name.cmp(&b.name));

            if fmt.json_mode {
                fmt.print_json(&items);
            } else {
                fmt.header(&format!("Items ({})", items.len()));
                for item in &items {
                    println!(
                        "  {:<40} [{:<10}] sink:{:>6}  {}",
                        item.name,
                        item.category,
                        item.sink_value,
                        if item.is_raw { "(raw)" } else { "" }
                    );
                }
            }
        }

        ListTarget::Recipes { item, alternate } => {
            let recipes: Vec<_> = db
                .all_recipes()
                .filter(|r| {
                    let alt_ok = !alternate || r.is_alternate;
                    let item_ok = item
                        .as_deref()
                        .map(|id| r.outputs.iter().any(|o| o.item == id))
                        .unwrap_or(true);
                    alt_ok && item_ok
                })
                .collect();

            if fmt.json_mode {
                fmt.print_json(&recipes);
            } else {
                fmt.header(&format!("Recipes ({})", recipes.len()));
                for r in &recipes {
                    let inputs: Vec<String> = r
                        .inputs
                        .iter()
                        .map(|i| format!("{}×{}", i.amount, i.item))
                        .collect();
                    let outputs: Vec<String> = r
                        .outputs
                        .iter()
                        .map(|o| format!("{}×{}", o.amount, o.item))
                        .collect();
                    println!(
                        "  {:<50} [{}] {}→{}",
                        r.name,
                        r.machine,
                        inputs.join("+"),
                        outputs.join("+")
                    );
                }
            }
        }

        ListTarget::Machines => {
            let mut machines: Vec<_> = db.all_machines().collect();
            machines.sort_by(|a, b| a.name.cmp(&b.name));

            if fmt.json_mode {
                fmt.print_json(&machines);
            } else {
                fmt.header(&format!("Machines ({})", machines.len()));
                for m in &machines {
                    println!(
                        "  {:<35} [{:<12}] {:.0} MW  in:{} out:{}",
                        m.name, m.category, m.power_mw, m.input_slots, m.output_slots
                    );
                }
            }
        }

        ListTarget::Resources { item } => {
            let mut resources: Vec<_> = if let Some(id) = &item {
                db.resources_for_item(id)
            } else {
                db.all_resources().collect()
            };
            resources.sort_by(|a, b| a.item.cmp(&b.item).then(a.purity.cmp(&b.purity)));

            if fmt.json_mode {
                fmt.print_json(resources.as_slice());
            } else {
                fmt.header(&format!("Resource Nodes ({})", resources.len()));
                for r in &resources {
                    println!(
                        "  {:<20} {:<8} {:>2} nodes  {:>7.1}/min max per node",
                        r.item, r.purity, r.node_count, r.max_rate_per_node
                    );
                }
            }
        }

        ListTarget::Belts => {
            let belts = db.conveyor_belts();
            if fmt.json_mode {
                fmt.print_json(belts);
            } else {
                fmt.header("Conveyor Belts");
                for b in belts {
                    println!(
                        "  {:<25} {:>5}/min  (Tier {} — {})",
                        b.name, b.rate_per_min, b.unlock_tier, b.unlock_milestone
                    );
                }
            }
        }

        ListTarget::Pipes => {
            let pipes = db.pipelines();
            if fmt.json_mode {
                fmt.print_json(pipes);
            } else {
                fmt.header("Pipelines");
                for p in pipes {
                    println!(
                        "  {:<25} {:>5} m³/min  (Tier {} — {})",
                        p.name, p.rate_per_min, p.unlock_tier, p.unlock_milestone
                    );
                }
            }
        }

        ListTarget::Milestones { tier } => {
            let tiers: Vec<_> = db
                .hub_tiers()
                .iter()
                .filter(|t| tier.is_none_or(|n| t.tier == n))
                .collect();
            if fmt.json_mode {
                fmt.print_json(&tiers);
            } else {
                for t in &tiers {
                    fmt.header(&format!("Tier {} — {}", t.tier, t.name));
                    for m in &t.milestones {
                        let cost: Vec<String> = m
                            .cost
                            .iter()
                            .map(|c| format!("{}×{}", c.amount, c.item))
                            .collect();
                        println!("  {:<35} [{}]", m.name, cost.join(", "));
                        if !m.unlocks_machines.is_empty() {
                            println!("    machines: {}", m.unlocks_machines.join(", "));
                        }
                        if !m.unlocks_recipes.is_empty() {
                            println!("    recipes:  {}", m.unlocks_recipes.join(", "));
                        }
                        if !m.unlocks_other.is_empty() {
                            println!("    other:    {}", m.unlocks_other.join(", "));
                        }
                    }
                }
            }
        }

        ListTarget::SpaceElevator => {
            let phases = db.space_elevator_phases();
            if fmt.json_mode {
                fmt.print_json(phases);
            } else {
                fmt.header("Space Elevator Phases");
                for p in phases {
                    let reqs: Vec<String> = p
                        .requirements
                        .iter()
                        .map(|r| format!("{}×{}", r.amount, r.item))
                        .collect();
                    println!(
                        "  Phase {} — {:<25} [{}]  → {}",
                        p.phase,
                        p.name,
                        reqs.join(", "),
                        p.unlocks
                    );
                }
            }
        }

        ListTarget::Mam { tree } => {
            let trees: Vec<_> = db
                .mam_trees()
                .iter()
                .filter(|t| tree.as_deref().is_none_or(|id| t.id == id))
                .collect();
            if fmt.json_mode {
                fmt.print_json(&trees);
            } else {
                for t in &trees {
                    fmt.header(&format!("MAM — {}", t.name));
                    for node in &t.nodes {
                        let cost: Vec<String> = node
                            .cost
                            .iter()
                            .map(|c| format!("{}×{}", c.amount, c.item))
                            .collect();
                        println!(
                            "  {:<35} ({})  [{}]  → {}",
                            node.name,
                            node.id,
                            cost.join(", "),
                            node.unlocks.join(", ")
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// calc
// ---------------------------------------------------------------------------

fn cmd_calc(db: &Db, fmt: &Formatter, query: &str, rate: f64, clock: f64) -> Result<()> {
    let recipe = db
        .find_recipe(query)
        .ok_or_else(|| anyhow::anyhow!("No recipe found for '{}'", query))?;

    if recipe.cycle_time_s <= 0.0 {
        bail!(
            "Recipe '{}' is an equipment workshop recipe and cannot be calculated",
            recipe.id
        );
    }

    let primary_output = recipe
        .outputs
        .first()
        .map(|o| o.item.as_str())
        .unwrap_or("");

    let machine = db.machine(&recipe.machine);
    let base_power = machine.map(|m| m.power_mw).unwrap_or(0.0);
    let machine_name = machine.map(|m| m.name.as_str()).unwrap_or(&recipe.machine);

    let result = calculate(recipe, primary_output, rate, clock, base_power);

    if fmt.json_mode {
        fmt.print_json(&json!({
            "recipe": result.recipe_id,
            "target_rate": result.target_rate,
            "machines": result.machines_exact,
            "clock_speed": result.clock_speed,
            "inputs": result.inputs.iter().map(|i| json!({"item": i.item, "rate": i.rate})).collect::<Vec<_>>(),
            "outputs": result.outputs.iter().map(|o| json!({"item": o.item, "rate": o.rate})).collect::<Vec<_>>(),
            "power_mw": result.power_mw,
        }));
    } else {
        fmt.header(&format!("calc: {}", result.recipe_name));
        fmt.separator();
        fmt.field(
            "Recipe",
            &format!("{} ({})", result.recipe_name, machine_name),
        );
        fmt.field(
            "Target",
            &format!("{} {}/min", result.target_rate, primary_output),
        );
        fmt.field(
            "Machines",
            &format!(
                "{:.3}x {} @ {:.0}% clock  ({} real)",
                result.machines_exact,
                machine_name,
                result.clock_speed * 100.0,
                result.machines_exact.ceil() as u64
            ),
        );
        for inp in &result.inputs {
            fmt.field("Input", &format!("{:.3}/min  {}", inp.rate, inp.item));
        }
        for out in &result.outputs {
            fmt.field("Output", &format!("{:.3}/min  {}", out.rate, out.item));
        }
        fmt.field("Power", &format!("{:.2} MW", result.power_mw));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// chain
// ---------------------------------------------------------------------------

fn cmd_chain(
    db: &Db,
    fmt: &Formatter,
    item_query: &str,
    rate: f64,
    no_alternates: bool,
    assume_args: &[String],
) -> Result<()> {
    let lower = item_query.to_lowercase();
    let item_id = db
        .all_items()
        .find(|i| i.id == lower || i.name.to_lowercase() == lower)
        .map(|i| i.id.clone())
        .ok_or_else(|| anyhow::anyhow!("Unknown item '{}'", item_query))?;

    let mut assumed_supplies: HashMap<String, f64> = HashMap::new();
    for arg in assume_args {
        let parts: Vec<&str> = arg.splitn(2, ':').collect();
        if parts.len() != 2 {
            bail!("--assume format must be ITEM:RATE, got '{}'", arg);
        }
        let supply_rate: f64 = parts[1]
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid rate in --assume '{}'", arg))?;
        assumed_supplies.insert(parts[0].to_string(), supply_rate);
    }

    let opts = ChainOptions {
        assumed_supplies,
        no_alternates,
    };
    let mut visited = std::collections::HashSet::new();
    let tree = resolve_chain(db, &item_id, rate, &opts, &mut visited);

    if fmt.json_mode {
        fmt.print_json(&chain_node_to_json(&tree));
    } else {
        print_chain_node(db, &tree, 0);
    }

    Ok(())
}

fn print_chain_node(db: &Db, node: &ChainNode, depth: usize) {
    let indent = "  ".repeat(depth);
    let item_name = db
        .item(&node.item_id)
        .map(|i| i.name.as_str())
        .unwrap_or(&node.item_id);

    if node.assumed {
        println!("{}[assumed] {:.2}/min  {}", indent, node.rate, item_name);
        return;
    }

    match &node.calc {
        None => {
            println!("{}[raw] {:.2}/min  {}", indent, node.rate, item_name);
        }
        Some(calc) => {
            println!(
                "{}{:.2}/min  {}  →  {:.3}x {}",
                indent, node.rate, item_name, calc.machines_exact, calc.machine_id
            );
        }
    }

    for child in &node.children {
        print_chain_node(db, child, depth + 1);
    }
}

fn chain_node_to_json(node: &ChainNode) -> serde_json::Value {
    json!({
        "item": node.item_id,
        "rate": node.rate,
        "assumed": node.assumed,
        "recipe": node.calc.as_ref().map(|c| &c.recipe_id),
        "machines": node.calc.as_ref().map(|c| c.machines_exact),
        "machine": node.calc.as_ref().map(|c| &c.machine_id),
        "power_mw": node.calc.as_ref().map(|c| c.power_mw),
        "children": node.children.iter().map(chain_node_to_json).collect::<Vec<_>>(),
    })
}

// ---------------------------------------------------------------------------
// overclock
// ---------------------------------------------------------------------------

fn cmd_overclock(db: &Db, fmt: &Formatter, query: &str, machines: u64, rate: f64) -> Result<()> {
    let recipe = db
        .find_recipe(query)
        .ok_or_else(|| anyhow::anyhow!("No recipe found for '{}'", query))?;

    if recipe.cycle_time_s <= 0.0 {
        bail!(
            "Recipe '{}' cannot be calculated (equipment workshop recipe)",
            recipe.id
        );
    }

    let primary_output = recipe
        .outputs
        .first()
        .map(|o| o.item.as_str())
        .unwrap_or("");
    let machine = db.machine(&recipe.machine);
    let base_power = machine.map(|m| m.power_mw).unwrap_or(0.0);
    let machine_name = machine.map(|m| m.name.as_str()).unwrap_or(&recipe.machine);

    let result = overclock(recipe, primary_output, machines, rate, base_power);

    if fmt.json_mode {
        fmt.print_json(&json!({
            "recipe": result.recipe_id,
            "machine": result.machine_id,
            "machine_count": result.machine_count,
            "target_rate": result.target_rate,
            "clock_speed": result.clock_speed,
            "feasible": result.feasible,
            "machines_at_max_clock": result.machines_at_max_clock,
            "power_per_machine_mw": result.power_per_machine_mw,
            "total_power_mw": result.total_power_mw,
            "shards_per_machine": result.shards_per_machine,
            "total_shards": result.total_shards,
        }));
    } else {
        fmt.header(&format!("overclock: {}", result.recipe_name));
        fmt.separator();
        fmt.field(
            "Recipe",
            &format!("{} ({})", result.recipe_name, machine_name),
        );
        fmt.field(
            "Target",
            &format!(
                "{}/min with {} machines",
                result.target_rate, result.machine_count
            ),
        );
        if result.feasible {
            fmt.field(
                "Clock speed",
                &format!(
                    "{:.4}  ({:.2}%)",
                    result.clock_speed,
                    result.clock_speed * 100.0
                ),
            );
            fmt.field(
                "Power shards",
                &format!(
                    "{} per machine  ({} total)",
                    result.shards_per_machine, result.total_shards
                ),
            );
            fmt.field(
                "Power draw",
                &format!(
                    "{:.2} MW/machine  ({:.2} MW total)",
                    result.power_per_machine_mw, result.total_power_mw
                ),
            );
        } else {
            println!(
                "  ✗ Requires {:.2}x clock ({:.0}%) — exceeds 250% maximum.",
                result.clock_speed,
                result.clock_speed * 100.0
            );
            fmt.field(
                "Solution",
                &format!(
                    "Use {} machines at 250% (3 shards each)",
                    result.machines_at_max_clock
                ),
            );
            fmt.field(
                "Power draw",
                &format!(
                    "{:.2} MW/machine  ({:.2} MW total at 250%)",
                    result.power_per_machine_mw,
                    result.power_per_machine_mw * result.machines_at_max_clock as f64
                ),
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// sink
// ---------------------------------------------------------------------------

fn cmd_sink(
    db: &Db,
    fmt: &Formatter,
    item_query: Option<&str>,
    rate: Option<f64>,
    category: Option<&str>,
) -> Result<()> {
    if let Some(query) = item_query {
        let lower = query.to_lowercase();
        let item = db
            .all_items()
            .find(|i| i.id == lower || i.name.to_lowercase() == lower)
            .ok_or_else(|| anyhow::anyhow!("Unknown item '{}'", query))?;

        if item.sink_value == 0 {
            if fmt.json_mode {
                fmt.print_json(&json!({ "item": item.id, "name": item.name, "sinkable": false }));
            } else {
                println!("  {} cannot be sinked (sink value = 0)", item.name);
            }
            return Ok(());
        }

        let points_per_min = rate.map(|r| r * item.sink_value as f64);

        if fmt.json_mode {
            fmt.print_json(&json!({
                "item": item.id,
                "name": item.name,
                "sink_value": item.sink_value,
                "rate": rate,
                "points_per_min": points_per_min,
            }));
        } else {
            fmt.header(&format!("sink: {}", item.name));
            fmt.separator();
            fmt.field("Sink value", &format!("{} points/item", item.sink_value));
            if let (Some(r), Some(ppm)) = (rate, points_per_min) {
                fmt.field("Rate", &format!("{}/min", r));
                fmt.field("Points/min", &format!("{:.0}", ppm));
            }
        }
        return Ok(());
    }

    // List mode: all sinkable items, optionally filtered by category
    let mut items: Vec<_> = db
        .all_items()
        .filter(|i| i.sink_value > 0)
        .filter(|i| category.is_none_or(|c| i.category == c))
        .collect();
    items.sort_by(|a, b| b.sink_value.cmp(&a.sink_value));

    if fmt.json_mode {
        let out: Vec<_> = items
            .iter()
            .map(|i| {
                json!({
                    "item": i.id,
                    "name": i.name,
                    "category": i.category,
                    "sink_value": i.sink_value,
                })
            })
            .collect();
        fmt.print_json(&out);
    } else {
        fmt.header(&format!("Sinkable items ({})", items.len()));
        for item in &items {
            println!(
                "  {:<40} [{:<10}] {:>8} pts",
                item.name, item.category, item.sink_value
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// nuclear
// ---------------------------------------------------------------------------

fn cmd_nuclear(fmt: &Formatter, plants: u64, fuel: &str) -> Result<()> {
    // Per-plant constants at 100% clock
    let (rods_per_min, waste_item, waste_per_min, waste_processing_hint) = match fuel {
        "plutonium" => (
            0.1_f64,
            "plutonium_waste",
            1.0_f64,
            "pathfinder chain ficsonium --rate <waste/min> --assume plutonium_waste:<waste/min>",
        ),
        _ => (
            0.2_f64,
            "uranium_waste",
            10.0_f64,
            "pathfinder chain plutonium_fuel_rod --rate <target> --assume uranium_waste:<waste/min>",
        ),
    };

    let fuel_label = if fuel == "plutonium" {
        "Plutonium Fuel Rod"
    } else {
        "Uranium Fuel Rod"
    };
    let power_mw_per_plant = 2500.0_f64;
    let water_per_plant = 240.0_f64;

    let total_rods = rods_per_min * plants as f64;
    let total_waste = waste_per_min * plants as f64;
    let total_power = power_mw_per_plant * plants as f64;
    let total_water = water_per_plant * plants as f64;
    // Water extractors at 100% clock produce 120 m³/min each
    let water_extractors = (total_water / 120.0).ceil() as u64;

    if fmt.json_mode {
        fmt.print_json(&json!({
            "plants": plants,
            "fuel": fuel,
            "fuel_rods_per_min": total_rods,
            "waste_item": waste_item,
            "waste_per_min": total_waste,
            "power_output_mw": total_power,
            "water_required_m3_per_min": total_water,
            "water_extractors_needed": water_extractors,
            "waste_processing_command": waste_processing_hint.replace("<waste/min>", &total_waste.to_string()),
        }));
    } else {
        fmt.header(&format!(
            "nuclear: {} × Nuclear Power Plant ({})",
            plants, fuel_label
        ));
        fmt.separator();
        fmt.field(
            "Power output",
            &format!(
                "{:.0} MW total  ({:.0} MW/plant)",
                total_power, power_mw_per_plant
            ),
        );
        fmt.field(
            &format!("{} consumed", fuel_label),
            &format!("{:.2}/min total", total_rods),
        );
        fmt.field(
            "Water required",
            &format!(
                "{:.0} m³/min  →  {} Water Extractors at 100%",
                total_water, water_extractors
            ),
        );
        fmt.field(
            &format!("{} produced", waste_item),
            &format!("{:.0}/min total", total_waste),
        );
        fmt.separator();
        println!("  To plan waste processing chain, run:");
        println!(
            "    {}",
            waste_processing_hint.replace("<waste/min>", &format!("{:.0}", total_waste))
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// companion
// ---------------------------------------------------------------------------

const AGENT_CONTENT: &str = include_str!("../../../agent/satisfactory-companion.md");
const AGENT_FILENAME: &str = "satisfactory-companion.md";

fn cmd_companion(fmt: &Formatter, action: CompanionAction) -> Result<()> {
    match action {
        CompanionAction::Install { global } => {
            let agents_dir = if global {
                let home = std::env::var_os("HOME")
                    .or_else(|| std::env::var_os("USERPROFILE"))
                    .map(std::path::PathBuf::from)
                    .ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?;
                home.join(".claude").join("agents")
            } else {
                std::path::PathBuf::from(".claude").join("agents")
            };

            std::fs::create_dir_all(&agents_dir)
                .with_context(|| format!("failed to create {}", agents_dir.display()))?;

            let dest = agents_dir.join(AGENT_FILENAME);
            std::fs::write(&dest, AGENT_CONTENT)
                .with_context(|| format!("failed to write {}", dest.display()))?;

            if fmt.json_mode {
                fmt.print_json(&json!({ "installed": dest.to_string_lossy() }));
            } else {
                println!("Companion agent installed to {}", dest.display());
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// progress
// ---------------------------------------------------------------------------

fn cmd_progress(fmt: &Formatter, path: &std::path::Path, action: ProgressAction) -> Result<()> {
    match action {
        ProgressAction::Show => {
            let state = progress::load(path)?;
            if fmt.json_mode {
                fmt.print_json(&state);
            } else {
                fmt.header("World Progress");
                fmt.separator();
                fmt.field("Milestones unlocked", &state.milestones.len().to_string());
                if !state.milestones.is_empty() {
                    for id in &state.milestones {
                        println!("    {id}");
                    }
                }
                fmt.field("MAM nodes researched", &state.mam_nodes.len().to_string());
                if !state.mam_nodes.is_empty() {
                    for id in &state.mam_nodes {
                        println!("    {id}");
                    }
                }
                fmt.field(
                    "Space Elevator phases",
                    &if state.space_elevator_phases.is_empty() {
                        "none".to_string()
                    } else {
                        state
                            .space_elevator_phases
                            .iter()
                            .map(|p| p.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    },
                );
                fmt.field(
                    "Alternate recipes found",
                    &state.alternate_recipes.len().to_string(),
                );
                if !state.alternate_recipes.is_empty() {
                    for id in &state.alternate_recipes {
                        println!("    {id}");
                    }
                }
            }
            Ok(())
        }

        ProgressAction::Unlock { target } => {
            let mut state = progress::load(path)?;
            match target {
                ProgressTarget::Milestone { id } => {
                    if state.milestones.contains(&id) {
                        if fmt.json_mode {
                            fmt.print_json(&serde_json::json!({ "milestone": id, "status": "already_unlocked" }));
                        } else {
                            println!("Milestone '{id}' is already unlocked.");
                        }
                        return Ok(());
                    }
                    state.milestones.push(id.clone());
                    state.milestones.sort();
                    progress::save(path, &state)?;
                    if fmt.json_mode {
                        fmt.print_json(
                            &serde_json::json!({ "milestone": id, "status": "unlocked" }),
                        );
                    } else {
                        println!("Milestone '{id}' unlocked.");
                    }
                }
                ProgressTarget::Mam { id } => {
                    if state.mam_nodes.contains(&id) {
                        if fmt.json_mode {
                            fmt.print_json(&serde_json::json!({ "mam_node": id, "status": "already_unlocked" }));
                        } else {
                            println!("MAM node '{id}' is already researched.");
                        }
                        return Ok(());
                    }
                    state.mam_nodes.push(id.clone());
                    state.mam_nodes.sort();
                    progress::save(path, &state)?;
                    if fmt.json_mode {
                        fmt.print_json(
                            &serde_json::json!({ "mam_node": id, "status": "unlocked" }),
                        );
                    } else {
                        println!("MAM node '{id}' researched.");
                    }
                }
                ProgressTarget::Phase { number } => {
                    if !(1..=5).contains(&number) {
                        bail!("Phase number must be between 1 and 5, got {number}");
                    }
                    if state.space_elevator_phases.contains(&number) {
                        if fmt.json_mode {
                            fmt.print_json(&serde_json::json!({ "phase": number, "status": "already_unlocked" }));
                        } else {
                            println!("Space Elevator phase {number} is already submitted.");
                        }
                        return Ok(());
                    }
                    state.space_elevator_phases.push(number);
                    state.space_elevator_phases.sort();
                    progress::save(path, &state)?;
                    if fmt.json_mode {
                        fmt.print_json(
                            &serde_json::json!({ "phase": number, "status": "unlocked" }),
                        );
                    } else {
                        println!("Space Elevator phase {number} submitted.");
                    }
                }
                ProgressTarget::Alt { id } => {
                    if !id.starts_with("alt_") {
                        bail!("Alternate recipe id must start with 'alt_', got '{id}'");
                    }
                    if state.alternate_recipes.contains(&id) {
                        if fmt.json_mode {
                            fmt.print_json(&serde_json::json!({ "alt_recipe": id, "status": "already_unlocked" }));
                        } else {
                            println!("Alternate recipe '{id}' is already recorded.");
                        }
                        return Ok(());
                    }
                    state.alternate_recipes.push(id.clone());
                    state.alternate_recipes.sort();
                    progress::save(path, &state)?;
                    if fmt.json_mode {
                        fmt.print_json(
                            &serde_json::json!({ "alt_recipe": id, "status": "unlocked" }),
                        );
                    } else {
                        println!("Alternate recipe '{id}' recorded.");
                    }
                }
            }
            Ok(())
        }

        ProgressAction::Lock { target } => {
            let mut state = progress::load(path)?;
            match target {
                ProgressTarget::Milestone { id } => {
                    if !state.milestones.contains(&id) {
                        if fmt.json_mode {
                            fmt.print_json(
                                &serde_json::json!({ "milestone": id, "status": "not_found" }),
                            );
                        } else {
                            println!("Milestone '{id}' was not unlocked.");
                        }
                        return Ok(());
                    }
                    state.milestones.retain(|m| m != &id);
                    progress::save(path, &state)?;
                    if fmt.json_mode {
                        fmt.print_json(&serde_json::json!({ "milestone": id, "status": "locked" }));
                    } else {
                        println!("Milestone '{id}' locked (removed).");
                    }
                }
                ProgressTarget::Mam { id } => {
                    if !state.mam_nodes.contains(&id) {
                        if fmt.json_mode {
                            fmt.print_json(
                                &serde_json::json!({ "mam_node": id, "status": "not_found" }),
                            );
                        } else {
                            println!("MAM node '{id}' was not researched.");
                        }
                        return Ok(());
                    }
                    state.mam_nodes.retain(|m| m != &id);
                    progress::save(path, &state)?;
                    if fmt.json_mode {
                        fmt.print_json(&serde_json::json!({ "mam_node": id, "status": "locked" }));
                    } else {
                        println!("MAM node '{id}' locked (removed).");
                    }
                }
                ProgressTarget::Phase { number } => {
                    if !state.space_elevator_phases.contains(&number) {
                        if fmt.json_mode {
                            fmt.print_json(
                                &serde_json::json!({ "phase": number, "status": "not_found" }),
                            );
                        } else {
                            println!("Space Elevator phase {number} was not submitted.");
                        }
                        return Ok(());
                    }
                    state.space_elevator_phases.retain(|p| p != &number);
                    progress::save(path, &state)?;
                    if fmt.json_mode {
                        fmt.print_json(&serde_json::json!({ "phase": number, "status": "locked" }));
                    } else {
                        println!("Space Elevator phase {number} locked (removed).");
                    }
                }
                ProgressTarget::Alt { id } => {
                    if !state.alternate_recipes.contains(&id) {
                        if fmt.json_mode {
                            fmt.print_json(
                                &serde_json::json!({ "alt_recipe": id, "status": "not_found" }),
                            );
                        } else {
                            println!("Alternate recipe '{id}' was not recorded.");
                        }
                        return Ok(());
                    }
                    state.alternate_recipes.retain(|r| r != &id);
                    progress::save(path, &state)?;
                    if fmt.json_mode {
                        fmt.print_json(
                            &serde_json::json!({ "alt_recipe": id, "status": "locked" }),
                        );
                    } else {
                        println!("Alternate recipe '{id}' locked (removed).");
                    }
                }
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// bottleneck
// ---------------------------------------------------------------------------

fn cmd_bottleneck(
    db: &Db,
    fmt: &Formatter,
    factory_path: Option<PathBuf>,
    world_path: Option<PathBuf>,
) -> Result<()> {
    let factories = if let Some(path) = factory_path {
        vec![serde_json::from_str::<pathfinder_core::Factory>(
            &std::fs::read_to_string(&path)?,
        )?]
    } else if let Some(path) = world_path {
        load_factories(&path)?
    } else {
        bail!("Provide --factory or --world");
    };

    let mut all_issues = Vec::new();
    for factory in &factories {
        let issues = analyse_factory(db, factory);
        all_issues.extend(issues);
    }

    if fmt.json_mode {
        let json_issues: Vec<_> = all_issues
            .iter()
            .map(|i| {
                json!({
                    "severity": i.severity.to_string(),
                    "factory": i.factory_id,
                    "machine_group": i.machine_group_id,
                    "message": i.message,
                })
            })
            .collect();
        fmt.print_json(&json_issues);
    } else {
        if all_issues.is_empty() {
            println!("No issues found.");
        } else {
            for issue in &all_issues {
                println!(
                    "[{}] {}/{}: {}",
                    issue.severity, issue.factory_id, issue.machine_group_id, issue.message
                );
            }
        }
    }

    Ok(())
}
