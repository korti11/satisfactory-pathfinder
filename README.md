# Satisfactory Pathfinder

A CLI factory planning companion for [Satisfactory](https://www.satisfactorygame.com/). Calculate production rates, resolve full supply chains, analyze factory bottlenecks, and plan builds — with an optional Claude Code agent that turns natural language into precise game calculations.

## Overview

Pathfinder has two components:

| Component | What it is |
|-----------|-----------|
| `pathfinder` CLI | A fast Rust binary for querying game data and running factory math |
| `satisfactory-companion` agent | A Claude Code subagent that uses the CLI to answer factory planning questions in natural language |

You can use the CLI on its own or pair it with the agent for a conversational planning experience.

---

## Installation

### Homebrew (macOS and Linux)

```bash
brew tap korti11/tap
brew install satisfactory-pathfinder
```

### Shell script (macOS and Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/korti11/satisfactory-pathfinder/master/install/install.sh | bash
```

Installs to `~/.local/bin` by default. Override with `INSTALL_DIR`:

```bash
INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/korti11/satisfactory-pathfinder/master/install/install.sh | bash
```

### PowerShell (Windows)

```powershell
irm https://raw.githubusercontent.com/korti11/satisfactory-pathfinder/master/install/install.ps1 | iex
```

Installs to `%LOCALAPPDATA%\Programs\pathfinder` and adds it to your user PATH.

### Build from source

Requires [Rust](https://rustup.rs/) (stable).

```bash
git clone https://github.com/korti11/satisfactory-pathfinder.git
cd satisfactory-pathfinder
cargo build --release
```

The binary will be at `target/release/pathfinder`. Copy it somewhere on your PATH:

```bash
# Linux / macOS
cp target/release/pathfinder ~/.local/bin/pathfinder

# Windows (PowerShell)
Copy-Item target\release\pathfinder.exe "$env:LOCALAPPDATA\Programs\pathfinder\pathfinder.exe"
```

---

## CLI Usage

All commands support `--json` for machine-readable output.

### Browse game data

```bash
# List all items
pathfinder list items

# Filter by category: raw, liquid, gas, ingot, part, fuel, equipment, special
pathfinder list items --category ingot

# List recipes for an item
pathfinder list recipes --item iron_rod

# List alternate recipes only
pathfinder list recipes --alternate

# List all machines
pathfinder list machines
```

### Calculate machine rates

```bash
# How many machines and at what clock to produce 60 Iron Rods/min
pathfinder calc "Iron Rod" --rate 60

# With a specific clock speed (percentage)
pathfinder calc "Iron Rod" --rate 60 --clock 150
```

### Resolve a full production chain

```bash
# Full chain for 5 Computers/min
pathfinder chain "Computer" --rate 5

# Skip alternate recipes
pathfinder chain "Computer" --rate 5 --no-alternates

# Treat an item as externally supplied (won't recurse into it)
pathfinder chain "Computer" --rate 5 --assume iron_ingot:240
```

### Analyze a factory for bottlenecks

```bash
# Single factory file
pathfinder bottleneck --factory world/my_factory.json

# All factories in a world file
pathfinder bottleneck --world world/factories.json
```

### Overclock optimizer

Given a fixed number of machines, find the clock speed needed to hit a target rate:

```bash
pathfinder overclock "Iron Rod" --machines 3 --rate 45
pathfinder overclock "Computer" --machines 5 --rate 10
```

### Sink value calculator

```bash
# List all sinkable items ranked by AWESOME Sink point value
pathfinder sink

# Points/min for a specific item at a given rate
pathfinder sink --item Computer --rate 5

# Filter by category
pathfinder sink --category part
```

### Nuclear power planner

```bash
# Resource rates and waste output for 4 uranium plants
pathfinder nuclear --plants 4

# Plutonium fuel rod variant
pathfinder nuclear --plants 2 --fuel plutonium
```

---

## Claude Code Agent

The `agent/satisfactory-companion.md` file is a [Claude Code](https://claude.ai/code) subagent that wraps the pathfinder CLI with natural language skills including:

- Factory design and ASCII blueprint generation
- Power budget and resource node planning
- Belt, pipe, and pipeline pump planning
- Train logistics calculation
- Space Elevator progress tracking
- Nuclear waste management
- Hard Drive alternate recipe advisor
- Unlock advisor (milestones, MAM research, Space Elevator phases)
- World factory dependency graph
- AWESOME Sink value ranking
- Building material estimation

### Install the agent

Once `pathfinder` is installed and on your PATH, run:

```bash
# Install globally (available in all Claude Code projects)
pathfinder companion install --global

# Install for the current project only
pathfinder companion install
```

### Example prompts

```
Design me a factory for 10 Computers/min
How many trains do I need between my oil factory and main base?
Am I close to completing Space Elevator Phase 3?
Which Hard Drive alternates should I prioritize for my current setup?
I have 4 nuclear plants — how do I handle the waste?
Plan the pipeline pumps for a 130m vertical run
```

---

## Data Files

All game knowledge lives in `data/`. These JSON files are the source of truth for all calculations — nothing is hardcoded in the Rust logic.

| File | Contents |
|------|----------|
| `items.json` | 175 items with categories, stack sizes, and sink values |
| `recipes.json` | 252 recipes including ~100 alternates |
| `machines.json` | 24 machines with power draw and slot counts |
| `resources.json` | 37 resource node entries with purity and extraction rates |
| `logistics.json` | Conveyor belt (Mk.1–6) and pipeline (Mk.1–2) tier capacities |
| `milestones.json` | All HUB milestones, Space Elevator phases, and MAM research trees |

---

## Factory File Format

Pathfinder can analyze your factories if you track them in a JSON file:

```json
[
  {
    "id": "oil_factory_01",
    "name": "Oil Factory",
    "location": "Northern Oil Fields",
    "active": true,
    "machines": [
      {
        "id": "fuel_refineries",
        "machine": "refinery",
        "recipe": "fuel_default",
        "count": 6,
        "clock_speed": 1.0,
        "notes": "Line 1 — Fuel for generators"
      }
    ],
    "inputs": [
      { "item": "crude_oil", "rate": 480.0, "source": "2x pure nodes" }
    ],
    "outputs": [
      { "item": "fuel", "rate": 240.0, "destination": "12x Fuel Generator" }
    ],
    "notes": "Self-powered via on-site generators."
  }
]
```

---

## Building and Contributing

```bash
# Build
cargo build

# Run clippy (required before committing)
cargo clippy

# Validate data file integrity
node tools/validate_data.js
```

Releases follow [Semantic Versioning](https://semver.org/). All work happens on `main`; releases are tagged (`v0.1.0`, `v0.2.0`, etc.) and trigger CI builds.
