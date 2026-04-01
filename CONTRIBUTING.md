# Contributing to Satisfactory Pathfinder

Thank you for your interest in contributing! This document explains how to report bugs, suggest features, and submit code changes.

---

## Reporting Bugs

Please open a [GitHub issue](https://github.com/korti11/satisfactory-pathfinder/issues) and include:

- Your operating system and architecture
- Pathfinder version (`pathfinder --version`)
- The exact command you ran
- The output you received
- What you expected to happen instead

If the bug involves incorrect game data (wrong recipe amounts, missing items, etc.) please also note the in-game values you observed so we can verify against the wiki.

---

## Suggesting Features

Open a GitHub issue before starting any significant work. This avoids wasted effort if the feature doesn't fit the project's direction. For new CLI commands or agent skills, briefly describe:

- What problem it solves
- What the command or skill would look like from the user's perspective
- Any game data it would need that isn't already in `data/`

---

## Development Setup

**Prerequisites:**
- [Rust](https://rustup.rs/) (stable toolchain)
- [Node.js](https://nodejs.org/) (for running `tools/validate_data.js`)

**Build and run:**

```bash
git clone https://github.com/korti11/satisfactory-pathfinder.git
cd satisfactory-pathfinder
cargo build
./target/debug/pathfinder list items
```

**Run clippy before every commit — this is required:**

```bash
cargo clippy
```

**Validate data file integrity after any changes to `data/`:**

```bash
node tools/validate_data.js
```

---

## Code Conventions

- **Error handling:** use `anyhow::Result` in the CLI layer (`pathfinder-cli`); use `thiserror` for domain errors in the core library (`pathfinder-core`)
- **Clippy:** all clippy warnings must be resolved before a PR can be merged — no `#[allow(...)]` unless there is a strong documented reason
- **No hardcoded game data:** never hardcode item IDs, recipe IDs, or numeric game values in Rust — always load from the JSON data files
- **JSON flag:** the `--json` flag must be checked early and affect all output — every command must support machine-readable output
- **Formatting:** use `cargo fmt` before committing

---

## Data File Changes

All game knowledge lives in `data/*.json`. When changing these files:

1. Follow the existing schema — do not add or remove fields without updating the corresponding Rust structs in `crates/pathfinder-core/src/models.rs`
2. Run `node tools/validate_data.js` and resolve all errors before committing
3. Verify values against the [Satisfactory Wiki](https://satisfactory.wiki.gg/) — the wiki occasionally has errors so cross-check with in-game values when possible
4. Item and recipe IDs use `snake_case`; alternate recipe IDs are prefixed with `alt_`

---

## Pull Request Process

External contributors should use the standard GitHub fork workflow:

1. **Fork** the repository on GitHub
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/<your-username>/satisfactory-pathfinder.git
   ```
3. **Create a branch** for your change:
   ```bash
   git checkout -b feat/my-new-command
   ```
4. Make your changes, ensuring `cargo clippy` and `cargo build` pass cleanly
5. If your change touches `data/`, run `node tools/validate_data.js`
6. **Push** to your fork and open a Pull Request against `main` on this repository

**Commit message convention:**

| Prefix | Use for |
|--------|---------|
| `feat:` | New CLI command or agent skill |
| `fix:` | Bug fix |
| `data:` | Changes to `data/*.json` files |
| `docs:` | README, CONTRIBUTING, or other documentation |
| `chore:` | Build config, CI, dependencies, tooling |

Keep each PR focused — one feature or bug fix per PR. Write a clear description explaining what changed and why.

---

## Adding a New CLI Command

1. Add the core logic (if needed) to `crates/pathfinder-core/src/`
2. Add the subcommand to the `Commands` enum in `crates/pathfinder-cli/src/main.rs`
3. Implement the handler function following the existing `cmd_*` pattern
4. Support both human-readable and `--json` output
5. Run `cargo clippy` and fix all warnings
6. Update the usage examples in `README.md`

## Adding or Updating an Agent Skill

1. Edit `agent/satisfactory-companion.md`
2. If the skill calls a new CLI command, document the command in the "Using pathfinder" section
3. If the skill uses new game data, ensure that data exists in `data/` first
4. Update the `description` field in the frontmatter if the new skill covers a topic not already mentioned — this is what the parent Claude agent reads to decide when to invoke the companion
5. Copy the updated file to `~/.claude/agents/satisfactory-companion.md` to test it locally before submitting

---

## License

By contributing you agree that your contributions will be licensed under the [MIT License](LICENSE).
