# Wave 1 Foundation - Hive Mind Coordination

**Task:** Implement Scene Serialization (RON) and Configuration System (TOML)

## Agents

| Agent | Focus | Status |
|-------|-------|--------|
| Scene Agent | RON scene serialization | Complete |
| Config Agent | TOML config system | Running |

## Coordination Notes

### Shared Dependencies
Both agents will add `serde` dependencies. Expected merge conflict resolution:
- Keep both sets of dependencies
- `serde` should be added to workspace dependencies

### Key Files

**Scene Agent owns:**
- `crates/rust4d_math/` - Add serde derives to Vec4
- `crates/rust4d_core/` - Add serde to Transform4D, Material, create shapes.rs, scene.rs
- `scenes/` - Create RON scene files

**Config Agent owns:**
- `src/config.rs` - New config module
- `config/` - TOML config files
- Root `Cargo.toml` - Add figment dependency

### Communication Log

Use this section to leave notes for the other agent:

---

**Config Agent (2026-01-27):**
- I added figment/serde dependencies to Cargo.toml (committed)
- I created src/config.rs with AppConfig struct hierarchy
- I can't run `cargo check` or tests yet because rust4d_core has an error:
  - `entity.rs` imports `crate::shapes::ShapeTemplate` but `shapes` module isn't declared in lib.rs
  - The file `shapes.rs` exists, just needs `mod shapes;` and `pub use shapes::ShapeTemplate;` in lib.rs
- Once Scene Agent fixes that, I can test my config module
- I'm proceeding with creating the TOML config files

**Scene Agent (2026-01-27):**
- Task complete! All scene serialization is implemented and tested.
- Fixed the lib.rs exports that Config Agent mentioned - `mod shapes;` and exports are now in place
- 8 commits made with all changes
- All tests passing (124 tests total across rust4d_math and rust4d_core)
- The build should work now. Run `cargo build` to verify.
- Scene files created: `scenes/default.ron` and `scenes/test_chamber.ron`
- Report written to `scratchpad/reports/2026-01-27-wave-1-foundation/scene-agent-report.md`

