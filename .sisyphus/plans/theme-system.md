# Theme System Implementation

## TL;DR

> **Quick Summary**: Implement a TOML-based theme system for the AoE TUI, allowing users to select from built-in themes via settings. Leverages existing unused `ThemeConfig` infrastructure. Creates draft PR to upstream when complete.
> 
> **Deliverables**:
> - Theme loader with TOML parsing and hex color support
> - 3 built-in themes (phosphor, tokyo-night, catppuccin-latte) embedded at compile time
> - Settings TUI integration for theme selection
> - Profile-level theme override support (already wired, just needs activation)
> - Draft PR to upstream (njbrake/agent-of-empires) following contribution guidelines
> 
> **Estimated Effort**: Medium
> **Parallel Execution**: YES - 4 waves
> **Critical Path**: Task 1 (types) → Task 4 (loader) → Task 6 (wiring) → Task 9 (verification) → Task 10 (PR)

---

## Context

### Original Request
GitHub Issue #295: Add support for custom themes and color schemes for the TUI. User wants theme names mapped to definition files that ship with the application, following Rust TUI ecosystem best practices.

### Interview Summary
**Key Discussions**:
- TOML format preferred (consistent with existing config.toml)
- 17 existing color primitives must all be covered
- Versioning optional (user unsure if complexity worth it)
- Follow helix/gitui/zellij patterns

**Research Findings**:
- Helix pattern: TOML with `[palette]` section + semantic mappings
- Built-in themes embedded via `include_str!()` for zero runtime I/O
- Theme loader with fallback chain: configured name → default theme
- Existing `ThemeConfig` struct is UNUSED - infrastructure exists, just needs activation

### Metis Review
**Identified Gaps** (addressed):
- Error handling for invalid themes: Fall back to phosphor + log warning
- Hot reload vs restart: Apply immediately (matches existing settings pattern)
- All 17 fields required in theme TOML (no partial themes)
- User-defined themes explicitly OUT OF SCOPE for this PR

---

## Work Objectives

### Core Objective
Enable users to select TUI themes from Settings, with themes defined in TOML files embedded in the binary.

### Concrete Deliverables
- `src/tui/themes/mod.rs` - ThemeLoader with TOML parsing
- `src/tui/themes/phosphor.toml` - Current hardcoded theme as TOML
- `src/tui/themes/tokyo-night.toml` - Dark theme variant
- `src/tui/themes/catppuccin-latte.toml` - Light theme variant
- Updated `src/tui/styles.rs` - Serde derives for Theme struct
- Updated `src/tui/app.rs` - Wire config.theme.name to loader
- Updated `src/tui/settings/fields.rs` - Theme category and field
- Updated `src/tui/settings/input.rs` - Theme selection handling

### Definition of Done
- [ ] `cargo build --release` succeeds
- [ ] `cargo test` passes (all existing + new theme tests)
- [ ] `cargo clippy` clean
- [ ] User can change theme in Settings TUI
- [ ] Theme persists across app restarts
- [ ] Invalid theme name falls back to phosphor with log warning
- [ ] Draft PR opened on `upstream` (njbrake/agent-of-empires) from `origin:feat/themes`
- [ ] PR follows `.github/pull_request_template.md` exactly (AI Agent checkbox checked)

### Must Have
- Theme selection in Settings TUI
- At least 3 built-in themes
- Fallback to default on invalid theme name
- Immediate application (no restart required)
- Profile-level override support

### Must NOT Have (Guardrails)
- NO user-defined theme support (future PR)
- NO theme inheritance/extends mechanism
- NO partial theme files (all 17 fields required)
- NO theme editor in TUI (dropdown picker only)
- NO per-session themes (global setting only, profile overrides OK)
- NO theme preview dialog
- NO syntax highlighting themes (TUI chrome only)

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** - ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (cargo test)
- **Automated tests**: Tests-after
- **Framework**: Rust native tests (#[cfg(test)])

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Settings UI**: Use interactive_bash (tmux) to navigate settings and change theme
- **Config persistence**: Use Bash to check config.toml after theme change
- **Fallback behavior**: Use Bash to set invalid theme and verify logs

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation - can start immediately):
├── Task 1: Theme types + serde derives [quick]
├── Task 2: Theme TOML files (phosphor, tokyo-night, catppuccin-latte) [quick]
└── Task 3: Hex color parser utility [quick]

Wave 2 (Core implementation - depends on Wave 1):
├── Task 4: ThemeLoader with embedded themes (depends: 1, 2, 3) [unspecified-high]
├── Task 5: Settings TUI fields (depends: 1) [unspecified-high]
└── Task 6: App wiring + fallback logic (depends: 4) [unspecified-high]

Wave 3 (Integration + Tests):
├── Task 7: Settings input handling (depends: 5, 6) [quick]
├── Task 8: Unit tests for theme loading (depends: 4) [quick]
└── Task 9: Integration verification (depends: all) [unspecified-high]

Wave 4 (PR Creation - after verification):
└── Task 10: Create draft PR to upstream (depends: 9) [quick]

Wave FINAL (Independent review - 4 parallel):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: TUI QA with tmux (unspecified-high)
└── Task F4: Scope fidelity check (deep)

Critical Path: Task 1 → Task 4 → Task 6 → Task 7 → Task 9 → Task 10 → FINAL
Parallel Speedup: ~50% faster than sequential
Max Concurrent: 3 (Wave 1)
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|------------|--------|------|
| 1 | - | 4, 5 | 1 |
| 2 | - | 4 | 1 |
| 3 | - | 4 | 1 |
| 4 | 1, 2, 3 | 6, 8 | 2 |
| 5 | 1 | 7 | 2 |
| 6 | 4 | 7, 9 | 2 |
| 7 | 5, 6 | 9 | 3 |
| 8 | 4 | 9 | 3 |
| 9 | 7, 8 | 10 | 3 |
| 10 | 9 | FINAL | 4 |

### Agent Dispatch Summary

- **Wave 1**: 3 tasks → `quick` x3
- **Wave 2**: 3 tasks → `unspecified-high` x3
- **Wave 3**: 3 tasks → `quick` x2, `unspecified-high` x1
- **Wave 4**: 1 task → `quick` x1 (PR creation)
- **FINAL**: 4 tasks → `oracle` x1, `unspecified-high` x2, `deep` x1

---

## TODOs

- [x] 1. Theme Types and Serde Derives
- [x] 2. Create Theme TOML Files
- [x] 3. Hex Color Parser
- [x] 4. ThemeLoader
- [x] 5. Settings TUI Theme Fields
- [x] 6. App Wiring and Fallback Logic

  **What to do**:
  - Add `#[derive(Debug, Clone, Serialize, Deserialize)]` to `Theme` struct in `src/tui/styles.rs`
  - Create `ThemeColor` newtype wrapper around `ratatui::style::Color` with custom serde
  - Update all 17 fields from `Color` to `ThemeColor`
  - Keep `Theme::phosphor()` method as fallback (will be used by loader)

  **Must NOT do**:
  - Don't remove existing `Theme::phosphor()` method
  - Don't change the field names (must match TOML keys)
  - Don't add any theme loading logic here (that's Task 4)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Small, focused change to a single file with clear pattern
  - **Skills**: []
    - No special skills needed - straightforward Rust code

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3)
  - **Blocks**: Task 4, Task 5
  - **Blocked By**: None (can start immediately)

  **References**:
  - `src/tui/styles.rs:5-31` - Current Theme struct definition (17 Color fields)
  - `src/tui/styles.rs:39-63` - `Theme::phosphor()` implementation to preserve
  - Helix pattern: `#[derive(Debug, Clone, Serialize, Deserialize)]` on theme struct

  **WHY Each Reference Matters**:
  - `styles.rs:5-31` - Shows exact fields that need ThemeColor wrapper
  - `styles.rs:39-63` - Must keep working as hardcoded fallback

  **Acceptance Criteria**:
  - [ ] `Theme` struct has Serialize + Deserialize derives
  - [ ] `ThemeColor` newtype created with custom deserializer
  - [ ] `Theme::phosphor()` still compiles and works
  - [ ] `cargo check` passes

  **QA Scenarios**:
  ```
  Scenario: Theme struct serializes to TOML
    Tool: Bash (cargo test)
    Preconditions: New test added in styles.rs
    Steps:
      1. Run: cargo test theme_serialization --lib
      2. Test creates Theme::phosphor(), serializes to TOML string
      3. Verify TOML contains all 17 field keys
    Expected Result: Test passes, TOML output has background, border, text, etc.
    Evidence: .sisyphus/evidence/task-1-serialization.txt

  Scenario: ThemeColor parses hex colors
    Tool: Bash (cargo test)
    Preconditions: Unit test for ThemeColor deserialization
    Steps:
      1. Run: cargo test theme_color_parsing --lib
      2. Test parses "#39ff14" -> Color::Rgb(57, 255, 20)
      3. Test parses "#fff" shorthand -> Color::Rgb(255, 255, 255)
    Expected Result: Both hex formats parse correctly
    Evidence: .sisyphus/evidence/task-1-color-parsing.txt
  ```

  **Commit**: NO (groups with final commit)

- [x] 2. Create Theme TOML Files

  **What to do**:
  - Create `src/tui/themes/` directory
  - Create `phosphor.toml` with exact values from `Theme::phosphor()`
  - Create `tokyo-night.toml` with dark theme colors (based on tokyo-night palette)
  - Create `catppuccin-latte.toml` with light theme colors (based on catppuccin latte)
  - All 17 fields must be present in each file

  **Must NOT do**:
  - Don't add `[palette]` section (keep it simple, direct hex values)
  - Don't add `inherits` or any inheritance mechanism
  - Don't add version field (decided as out of scope)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: File creation with known content, no complex logic
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3)
  - **Blocks**: Task 4
  - **Blocked By**: None

  **References**:
  - `src/tui/styles.rs:41-62` - Exact RGB values for phosphor theme
  - https://github.com/enkia/tokyo-night-vscode-theme - Tokyo Night color palette reference
  - https://github.com/catppuccin/catppuccin - Catppuccin Latte palette reference

  **WHY Each Reference Matters**:
  - `styles.rs:41-62` - Source of truth for phosphor.toml values
  - External palettes - Ensure tokyo-night and catppuccin-latte are authentic

  **Acceptance Criteria**:
  - [ ] `src/tui/themes/phosphor.toml` exists with all 17 fields
  - [ ] `src/tui/themes/tokyo-night.toml` exists with all 17 fields
  - [ ] `src/tui/themes/catppuccin-latte.toml` exists with all 17 fields
  - [ ] All colors are valid hex format (#RRGGBB)

  **QA Scenarios**:
  ```
  Scenario: All theme files have required fields
    Tool: Bash (grep/toml parsing)
    Preconditions: Theme files created
    Steps:
      1. Run: for f in src/tui/themes/*.toml; do tomlq keys $f | wc -l; done
      2. Or: grep -c "=" src/tui/themes/phosphor.toml (should be 17)
    Expected Result: Each file has exactly 17 key-value pairs
    Evidence: .sisyphus/evidence/task-2-field-count.txt

  Scenario: Phosphor TOML matches hardcoded values
    Tool: Bash (comparison)
    Preconditions: phosphor.toml created
    Steps:
      1. Extract background value from phosphor.toml
      2. Compare with Color::Rgb(16, 20, 18) -> #101412
      3. Verify match
    Expected Result: background = "#101412" in TOML
    Evidence: .sisyphus/evidence/task-2-phosphor-match.txt
  ```

  **Commit**: NO (groups with final commit)

- [x] 3. Hex Color Parser Utility

  **What to do**:
  - Create `src/tui/themes/color.rs` with `parse_hex_color(s: &str) -> Result<Color>`
  - Support `#RRGGBB` format (6 hex digits)
  - Support `#RGB` shorthand (3 hex digits, expand to 6)
  - Return `anyhow::Result` with descriptive error messages
  - Implement as the custom deserializer for `ThemeColor`

  **Must NOT do**:
  - Don't add named color support ("red", "blue") - hex only for simplicity
  - Don't add alpha channel support
  - Don't use external crates (simple enough to implement)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Small utility function with clear spec
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2)
  - **Blocks**: Task 4
  - **Blocked By**: None

  **References**:
  - `ratatui::style::Color::Rgb(u8, u8, u8)` - Target type to construct
  - Helix hex parsing pattern (simple approach without dependencies)

  **WHY Each Reference Matters**:
  - Need to construct `Color::Rgb` from parsed hex values

  **Acceptance Criteria**:
  - [ ] `parse_hex_color("#39ff14")` returns `Ok(Color::Rgb(57, 255, 20))`
  - [ ] `parse_hex_color("#fff")` returns `Ok(Color::Rgb(255, 255, 255))`
  - [ ] `parse_hex_color("invalid")` returns descriptive error
  - [ ] `parse_hex_color("#gggggg")` returns error (invalid hex)

  **QA Scenarios**:
  ```
  Scenario: Valid hex colors parse correctly
    Tool: Bash (cargo test)
    Preconditions: parse_hex_color function implemented
    Steps:
      1. Run: cargo test hex_color_parsing --lib
      2. Test cases: "#000000", "#ffffff", "#39ff14", "#abc"
    Expected Result: All valid hex strings parse to correct RGB values
    Evidence: .sisyphus/evidence/task-3-valid-hex.txt

  Scenario: Invalid hex colors return errors
    Tool: Bash (cargo test)
    Preconditions: Error handling implemented
    Steps:
      1. Run: cargo test hex_color_errors --lib
      2. Test cases: "invalid", "#gg0000", "#12345", ""
    Expected Result: All return Err with message containing "invalid"
    Evidence: .sisyphus/evidence/task-3-invalid-hex.txt
  ```

  **Commit**: NO (groups with final commit)

- [x] 4. ThemeLoader with Embedded Themes

  **What to do**:
  - Create `src/tui/themes/mod.rs` with `ThemeLoader` struct
  - Use `include_str!()` to embed all 3 TOML files at compile time
  - Implement `load_theme(name: &str) -> Theme` that:
    - Looks up name in embedded themes map
    - Parses TOML to Theme struct
    - Falls back to phosphor on unknown name (with `tracing::warn!`)
  - Export `AVAILABLE_THEMES: &[&str]` for settings dropdown
  - Add `pub mod themes;` to `src/tui/mod.rs`

  **Must NOT do**:
  - Don't read from filesystem (all embedded)
  - Don't add user theme directory scanning
  - Don't cache parsed themes (parse on each load is fine for now)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Core module with TOML parsing, error handling, module wiring
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 6, Task 8
  - **Blocked By**: Tasks 1, 2, 3

  **References**:
  - `src/tui/styles.rs` - Theme struct to deserialize into (after Task 1)
  - `src/tui/themes/*.toml` - Files to embed (after Task 2)
  - `src/tui/themes/color.rs` - Hex parser (after Task 3)
  - `src/tui/mod.rs:1-20` - Module declarations to update
  - Helix pattern: `include_str!()` + HashMap for builtin themes

  **WHY Each Reference Matters**:
  - Must deserialize TOML into Theme struct
  - Must embed the TOML files created in Task 2
  - Must use hex parser from Task 3 via ThemeColor deserializer

  **Acceptance Criteria**:
  - [ ] `ThemeLoader::load("phosphor")` returns valid Theme
  - [ ] `ThemeLoader::load("tokyo-night")` returns valid Theme
  - [ ] `ThemeLoader::load("invalid-name")` returns phosphor + logs warning
  - [ ] `AVAILABLE_THEMES` contains ["phosphor", "tokyo-night", "catppuccin-latte"]
  - [ ] `cargo check` passes with new module

  **QA Scenarios**:
  ```
  Scenario: Load all built-in themes successfully
    Tool: Bash (cargo test)
    Preconditions: ThemeLoader implemented
    Steps:
      1. Run: cargo test theme_loader_builtin --lib
      2. Test loads each of 3 themes by name
      3. Verify each returns Theme with non-default colors
    Expected Result: All 3 themes load without error
    Evidence: .sisyphus/evidence/task-4-load-builtin.txt

  Scenario: Unknown theme falls back to phosphor
    Tool: Bash (cargo test)
    Preconditions: Fallback logic implemented
    Steps:
      1. Run: cargo test theme_loader_fallback --lib
      2. Load "nonexistent-theme"
      3. Verify returns phosphor colors
      4. Verify warning logged (check test captures logs)
    Expected Result: Returns phosphor theme, logs warning
    Evidence: .sisyphus/evidence/task-4-fallback.txt
  ```

  **Commit**: NO (groups with final commit)

- [x] 5. Settings TUI Theme Fields

  **What to do**:
  - Add `SettingsCategory::Theme` variant to enum in `src/tui/settings/fields.rs`
  - Add `FieldKey::ThemeName` variant
  - Create `build_theme_fields()` function returning theme field
  - Use `FieldType::Select` with options from `AVAILABLE_THEMES`
  - Wire into `build_fields_for_category()` match
  - Add case to `apply_field_to_global()` to update `config.theme.name`
  - Add case to `apply_field_to_profile()` for profile override

  **Must NOT do**:
  - Don't add theme preview
  - Don't add theme editor
  - Don't add multiple theme-related fields (just name selection)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Settings pattern is complex, needs careful wiring
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (partially)
  - **Parallel Group**: Wave 2 (with Task 4, 6)
  - **Blocks**: Task 7
  - **Blocked By**: Task 1 (needs Theme types)

  **References**:
  - `src/tui/settings/fields.rs:15-37` - SettingsCategory enum (add Theme variant)
  - `src/tui/settings/fields.rs:41-82` - FieldKey enum (add ThemeName variant)
  - `src/tui/settings/fields.rs:190-220` - `build_sound_fields()` as pattern (uses Select type)
  - `src/tui/settings/fields.rs:799-900` - `apply_field_to_global()` (add ThemeName case)
  - `src/tui/settings/fields.rs:903-1000` - `apply_field_to_profile()` (add ThemeName case)
  - `src/session/config.rs:107-111` - ThemeConfig struct (target of apply)

  **WHY Each Reference Matters**:
  - Enum variants must match existing pattern exactly
  - `build_sound_fields()` shows Select field pattern to follow
  - Apply functions show exact wiring pattern

  **Acceptance Criteria**:
  - [ ] `SettingsCategory::Theme` variant exists
  - [ ] `FieldKey::ThemeName` variant exists
  - [ ] Theme category appears in settings TUI
  - [ ] Dropdown shows 3 theme options
  - [ ] `cargo check` passes

  **QA Scenarios**:
  ```
  Scenario: Theme category appears in settings
    Tool: interactive_bash (tmux)
    Preconditions: AoE built with theme settings
    Steps:
      1. tmux new-session -d -s test-theme "cargo run --release"
      2. Wait 3s for app to start
      3. Send 's' key to open settings
      4. Navigate categories, look for "Theme"
      5. Capture screenshot
    Expected Result: Theme category visible in settings list
    Evidence: .sisyphus/evidence/task-5-settings-category.png

  Scenario: Theme dropdown shows all options
    Tool: interactive_bash (tmux)
    Preconditions: Theme settings wired
    Steps:
      1. Navigate to Theme category in settings
      2. Focus theme name field
      3. Open dropdown (Enter or space)
      4. Verify phosphor, tokyo-night, catppuccin-latte visible
    Expected Result: All 3 themes in dropdown
    Evidence: .sisyphus/evidence/task-5-dropdown.png
  ```

  **Commit**: NO (groups with final commit)

- [x] 6. App Wiring and Fallback Logic

  **What to do**:
  - Update `App::new()` in `src/tui/app.rs` to load theme from config
  - Replace `Theme::default()` with `ThemeLoader::load(&config.theme.name)`
  - Handle empty theme name: default to "phosphor"
  - Ensure theme applies to all components (already parameter-drilled)
  - Add method `App::set_theme(&mut self, name: &str)` for settings changes

  **Must NOT do**:
  - Don't change the parameter drilling pattern (it works)
  - Don't add theme caching beyond what's needed
  - Don't add live theme switching animations

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Core wiring with error handling, touches app initialization
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2 (after Task 4)
  - **Blocks**: Task 7, Task 9
  - **Blocked By**: Task 4

  **References**:
  - `src/tui/app.rs:56` - `theme: Theme` field in App struct
  - `src/tui/app.rs:81` - Current `Theme::default()` call to replace
  - `src/tui/app.rs:84` - `config` is loaded here, has `config.theme.name`
  - `src/tui/themes/mod.rs` - ThemeLoader to use (after Task 4)

  **WHY Each Reference Matters**:
  - Line 81 is exact location to change
  - Line 84 shows config is available at this point
  - Need to import ThemeLoader from themes module

  **Acceptance Criteria**:
  - [ ] App loads theme from `config.theme.name`
  - [ ] Empty theme name defaults to "phosphor"
  - [ ] `App::set_theme()` method exists for runtime changes
  - [ ] Theme change in settings immediately reflects in UI

  **QA Scenarios**:
  ```
  Scenario: App loads configured theme on startup
    Tool: Bash (config edit + run)
    Preconditions: Theme loader wired
    Steps:
      1. Edit ~/.config/agent-of-empires/config.toml: [theme] name = "tokyo-night"
      2. Run: cargo run --release
      3. Observe TUI colors match tokyo-night palette
    Expected Result: TUI uses tokyo-night colors (darker, blue accents)
    Evidence: .sisyphus/evidence/task-6-startup-theme.png

  Scenario: Invalid theme name falls back gracefully
    Tool: Bash (config + logs)
    Preconditions: Fallback logic in place
    Steps:
      1. Edit config.toml: [theme] name = "does-not-exist"
      2. Run: RUST_LOG=agent_of_empires=debug cargo run --release 2>&1 | head -50
      3. Check for warning log about unknown theme
      4. Verify app starts with phosphor colors
    Expected Result: Warning logged, phosphor theme used
    Evidence: .sisyphus/evidence/task-6-fallback-log.txt
  ```

  **Commit**: NO (groups with final commit)

- [x] 7. Settings Input Handling

  **What to do**:
  - Add `FieldKey::ThemeName` case to `handle_field_change()` in `src/tui/settings/input.rs`
  - When theme changes, call `app.set_theme(&new_value)`
  - Add `FieldKey::ThemeName` case to `clear_profile_override()` at line 402
  - Ensure config is saved after theme change (existing pattern)

  **Must NOT do**:
  - Don't add confirmation dialog for theme change
  - Don't add theme preview before applying

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Small addition following exact existing pattern
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 9
  - **Blocked By**: Tasks 5, 6

  **References**:
  - `src/tui/settings/input.rs:200-400` - `handle_field_change()` pattern
  - `src/tui/settings/input.rs:402-450` - `clear_profile_override()` cases
  - `src/tui/app.rs` - `App::set_theme()` method (after Task 6)

  **WHY Each Reference Matters**:
  - Must follow exact pattern in `handle_field_change()`
  - `clear_profile_override()` must handle new field type

  **Acceptance Criteria**:
  - [ ] Changing theme in settings applies immediately
  - [ ] Theme selection persists to config.toml
  - [ ] Profile override can be cleared
  - [ ] No restart required for theme change

  **QA Scenarios**:
  ```
  Scenario: Theme change applies immediately
    Tool: interactive_bash (tmux)
    Preconditions: Full theme system wired
    Steps:
      1. Start aoe in tmux
      2. Navigate to Settings > Theme
      3. Change from phosphor to tokyo-night
      4. Observe immediate color change in TUI
      5. Screenshot before and after
    Expected Result: Colors change without restart/exit
    Evidence: .sisyphus/evidence/task-7-immediate-change.png

  Scenario: Theme persists after restart
    Tool: Bash (restart test)
    Preconditions: Theme changed to tokyo-night
    Steps:
      1. Exit aoe (q)
      2. Check config.toml: grep "name" ~/.config/agent-of-empires/config.toml
      3. Restart aoe
      4. Verify still tokyo-night colors
    Expected Result: config.toml has name = "tokyo-night", colors persist
    Evidence: .sisyphus/evidence/task-7-persistence.txt
  ```

  **Commit**: NO (groups with final commit)

- [x] 8. Unit Tests for Theme Loading

  **What to do**:
  - Add tests in `src/tui/themes/mod.rs` under `#[cfg(test)]`
  - Test: all built-in themes parse successfully
  - Test: unknown theme returns fallback
  - Test: AVAILABLE_THEMES contains all expected names
  - Test: Theme fields are correctly deserialized from TOML

  **Must NOT do**:
  - Don't test UI interactions (that's QA scenarios)
  - Don't mock the embedded themes (test real ones)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Standard unit tests following Rust patterns
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Task 7)
  - **Blocks**: Task 9
  - **Blocked By**: Task 4

  **References**:
  - `src/tui/themes/mod.rs` - Module to add tests to (after Task 4)
  - `src/session/storage.rs:200-300` - Example test patterns in codebase
  - Rust test conventions: `#[cfg(test)]` module

  **WHY Each Reference Matters**:
  - Tests should be in same file as implementation
  - Follow existing test patterns in codebase

  **Acceptance Criteria**:
  - [ ] `cargo test theme` runs at least 5 tests
  - [ ] All tests pass
  - [ ] Coverage: load success, fallback, available themes list

  **QA Scenarios**:
  ```
  Scenario: All theme tests pass
    Tool: Bash (cargo test)
    Preconditions: Tests written
    Steps:
      1. Run: cargo test theme --lib -- --nocapture
      2. Capture output showing test names and results
    Expected Result: 5+ tests, all pass
    Evidence: .sisyphus/evidence/task-8-tests.txt
  ```

  **Commit**: NO (groups with final commit)

- [x] 9. Integration Verification

  **What to do**:
  - Run full test suite: `cargo test`
  - Run clippy: `cargo clippy -- -D warnings`
  - Run release build: `cargo build --release`
  - Manual verification: start app, change theme, verify persistence
  - Document any issues found and fix them

  **Must NOT do**:
  - Don't skip any verification step
  - Don't ignore clippy warnings

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Full integration verification, may need fixes
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (final task before review)
  - **Blocks**: Final Verification Wave
  - **Blocked By**: Tasks 7, 8

  **References**:
  - All files modified in Tasks 1-8
  - `AGENTS.md` - Build/test commands

  **WHY Each Reference Matters**:
  - Must verify entire feature works end-to-end

  **Acceptance Criteria**:
  - [ ] `cargo test` - all tests pass
  - [ ] `cargo clippy` - no warnings
  - [ ] `cargo build --release` - succeeds
  - [ ] App starts and theme selection works

  **QA Scenarios**:
  ```
  Scenario: Full test suite passes
    Tool: Bash
    Preconditions: All tasks complete
    Steps:
      1. Run: cargo test 2>&1
      2. Capture full output
    Expected Result: All tests pass, no failures
    Evidence: .sisyphus/evidence/task-9-tests.txt

  Scenario: Clippy clean
    Tool: Bash
    Preconditions: All code written
    Steps:
      1. Run: cargo clippy -- -D warnings 2>&1
      2. Capture output
    Expected Result: No warnings or errors
    Evidence: .sisyphus/evidence/task-9-clippy.txt

  Scenario: End-to-end theme workflow
    Tool: interactive_bash (tmux)
    Preconditions: Release build ready
    Steps:
      1. Build: cargo build --release
      2. Start: ./target/release/aoe
      3. Go to Settings > Theme
      4. Select catppuccin-latte (light theme)
      5. Verify light colors applied
      6. Exit and restart
      7. Verify still light theme
    Expected Result: Complete workflow succeeds
    Evidence: .sisyphus/evidence/task-9-e2e.png
  ```

  **Commit**: YES
  - Message: `feat(tui): add theme system with 3 built-in themes`
  - Files: All modified files
  - Pre-commit: `cargo test && cargo clippy`

- [x] 10. Create Draft PR to Upstream

  **What to do**:
  - Push branch `feat/themes` to origin: `git push -u origin feat/themes`
  - Create draft PR from `origin:feat/themes` to `upstream:main` using `gh pr create`
  - Fill out PR template from `.github/pull_request_template.md` exactly:
    - **Description**: "Add theme system with 3 built-in themes (phosphor, tokyo-night, catppuccin-latte). Users can select themes from Settings TUI. Closes #295."
    - **PR Type**: Check "New Feature"
    - **Checklist**: Check all 4 boxes (understand code, tests pass, docs updated, UI screenshot included)
    - **AI Usage**: Check "This is fully AI-generated"
    - **AI Model/Tool used**: "Claude (Anthropic) via OpenCode"
    - **I am an AI Agent**: Check this box
  - Include screenshot of theme selection in Settings TUI
  - Link to GitHub issue #295

  **Must NOT do**:
  - Don't create as ready for review (must be draft)
  - Don't skip any PR template sections
  - Don't forget to check "I am an AI Agent" box

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple git/gh commands with template filling
  - **Skills**: [`git-master`]
    - `git-master`: Git push and branch operations

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4 (after all implementation)
  - **Blocks**: Final Verification Wave
  - **Blocked By**: Task 9

  **References**:
  - `.github/pull_request_template.md` - PR template to follow exactly
  - `CONTRIBUTING.md:89-101` - PR submission guidelines
  - GitHub Issue #295 - Issue to link and close

  **WHY Each Reference Matters**:
  - PR template has mandatory checklist (PR will be closed if deleted)
  - CONTRIBUTING.md specifies what to include in PR description
  - Issue #295 is the feature request this implements

  **Acceptance Criteria**:
  - [ ] Branch pushed to origin
  - [ ] Draft PR created on upstream repo (njbrake/agent-of-empires)
  - [ ] PR title: "feat(tui): add theme system with 3 built-in themes"
  - [ ] PR body follows template exactly (all sections filled)
  - [ ] "I am an AI Agent" checkbox is checked
  - [ ] Screenshot of theme selection UI attached
  - [ ] Links to #295

  **QA Scenarios**:
  ```
  Scenario: PR created successfully
    Tool: Bash (gh pr view)
    Preconditions: PR creation command executed
    Steps:
      1. Run: gh pr view --json state,draft,title,body
      2. Verify state is "OPEN" and draft is true
      3. Verify title matches expected
      4. Verify body contains all template sections
    Expected Result: Draft PR exists with correct metadata
    Evidence: .sisyphus/evidence/task-10-pr-created.txt

  Scenario: PR template compliance
    Tool: Bash (gh pr view)
    Preconditions: PR exists
    Steps:
      1. Run: gh pr view --json body | jq -r .body
      2. Grep for "I am an AI Agent" - must be checked [x]
      3. Grep for "New Feature" - must be checked [x]
      4. Grep for "#295" - must be linked
    Expected Result: All template requirements met
    Evidence: .sisyphus/evidence/task-10-template-compliance.txt
  ```

  **Commit**: NO (PR creation, not a code commit)

---

## Final Verification Wave

- [x] F1. **Plan Compliance Audit** - `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists. For each "Must NOT Have": search codebase for forbidden patterns - reject with file:line if found. Check evidence files exist in .sisyphus/evidence/. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** - `unspecified-high`
  Run `cargo check`, `cargo clippy`, `cargo test`. Review all changed files for: `unwrap()` without context, empty error handling, unused imports, dead code. Check for AI slop: excessive comments, over-abstraction.
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | VERDICT`

- [x] F3. **TUI QA with tmux** - `unspecified-high`
  Start `aoe` in tmux. Navigate to Settings. Change theme. Verify colors change immediately. Exit and restart. Verify theme persisted. Test invalid theme in config.toml manually - verify fallback works.
  Output: `Theme Selection [PASS/FAIL] | Persistence [PASS/FAIL] | Fallback [PASS/FAIL] | VERDICT`

- [x] F4. **Scope Fidelity Check** - `deep`
  Review all changes against plan. Verify no user-defined theme loading was added. Verify no theme inheritance. Verify no partial theme support. Verify all 3 themes have all 17 color fields. Flag any scope creep.
  Output: `Scope Boundaries [N/N respected] | Guardrails [N/N enforced] | VERDICT`

---

## Commit Strategy

Single feature commit after all tasks complete:

- **Commit**: `feat(tui): add theme system with 3 built-in themes`
  - Files: `src/tui/themes/`, `src/tui/styles.rs`, `src/tui/app.rs`, `src/tui/settings/fields.rs`, `src/tui/settings/input.rs`
  - Pre-commit: `cargo test && cargo clippy`

## PR Strategy

After commit, create draft PR to upstream:

- **Push**: `git push -u origin feat/themes`
- **PR**: `gh pr create --draft --repo njbrake/agent-of-empires --base main --head jerome-benoit:feat/themes`
- **Template**: Fill `.github/pull_request_template.md` exactly
- **AI Disclosure**: Check "This is fully AI-generated" + "I am an AI Agent"
- **Link**: Reference issue #295

---

## Success Criteria

### Verification Commands
```bash
cargo build --release  # Expected: success
cargo test            # Expected: all tests pass
cargo clippy          # Expected: no warnings
```

### Final Checklist
- [x] Theme selection works in Settings TUI
- [x] 3 themes available (phosphor, tokyo-night, catppuccin-latte)
- [x] Theme persists in config.toml
- [x] Invalid theme falls back to phosphor
- [x] All existing tests still pass
- [x] No clippy warnings
- [x] Draft PR opened on upstream (njbrake/agent-of-empires)
- [x] PR follows template exactly with "I am an AI Agent" checked
