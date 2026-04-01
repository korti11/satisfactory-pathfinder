## Description

What does this PR change and why?

## Type of change

- [ ] `feat:` New CLI command or agent skill
- [ ] `fix:` Bug fix
- [ ] `test:` Adding or updating tests
- [ ] `data:` Changes to `data/*.json` files
- [ ] `docs:` Documentation update
- [ ] `chore:` Build config, CI, dependencies, tooling

## Checklist

- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo test` passes
- [ ] If a new CLI command was added: integration test added in `crates/pathfinder-cli/tests/cli.rs`
- [ ] If `data/` was changed: `node tools/validate_data.js` passes and values verified against in-game values
- [ ] `README.md` updated if new commands or usage examples are needed
