# Pathfinder — Agent Instructions

## What this project is
A CLI tool (`pathfinder`) that acts as a companion to the Satisfactory pioneer, helping plan and optimize factories. It provides:
- `pathfinder list` — browse game data (items, recipes, machines)
- `pathfinder calc` — single machine rate calculations
- `pathfinder chain` — recursive full production chain resolution
- `pathfinder bottleneck` — analyze factory files for throughput problems

## Data files
All game knowledge lives in `data/*.json`. These are the source of truth.
Never answer questions about game data from memory — always read the JSON files.

## How to run commands (always use --json for programmatic use)
```bash
cargo run --bin pathfinder -- list items --json
cargo run --bin pathfinder -- calc "Iron Rod" --rate 30 --json
cargo run --bin pathfinder -- chain "Computer" --rate 5 --json
cargo run --bin pathfinder -- bottleneck --factory path/to/factory.json --json
```

## When helping with factory planning
1. Use `list recipes --item <item>` to find available recipes first
2. Use `calc` to verify single machine rates
3. Use `chain` to resolve full production requirements
4. Use `bottleneck` if a factory file is provided

## Code structure
- `crates/pathfinder-core/src/models.rs` — data structs
- `crates/pathfinder-core/src/db.rs` — JSON loading and lookup
- `crates/pathfinder-core/src/calculator.rs` — rate math
- `crates/pathfinder-core/src/chain.rs` — recursive chain resolver
- `crates/pathfinder-core/src/bottleneck.rs` — throughput analysis
- `crates/pathfinder-cli/src/main.rs` — CLI entry point

## Key calculation rules (Satisfactory game logic)
- Base recipe rate = (output_amount / cycle_time_s) * 60 → items per minute
- Clock speed scales both rate and power: rate * clock, power * clock^1.321928
- Machines needed = target_rate / (base_rate * clock_speed)
- Always round machines UP to nearest integer for real-world builds,
  but report the exact decimal for planning purposes

## Notes
- Always run `cargo clippy` and fix warnings before considering a task done
- Use `anyhow::Result` for error handling in the CLI layer; `thiserror` for domain errors in core
- The `--json` flag must be checked early and affect all downstream formatting
- Data file paths are configurable via `--data-dir` flag (default: `./data`)
- Never hardcode item or recipe IDs in Rust logic — always load from JSON
