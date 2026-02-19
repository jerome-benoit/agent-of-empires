# Decisions - Theme System Implementation

## [2026-02-19T10:53:00] Session Start

Architectural and design decisions made during implementation.

## Initial Decisions (from Plan)

- **Theme format**: TOML (consistent with config.toml)
- **Built-in themes**: 3 (phosphor, tokyo-night, catppuccin-latte)
- **Embedding**: `include_str!()` at compile time
- **Fallback**: phosphor theme on invalid/empty name
- **Hot reload**: Immediate application (no restart)
- **All 17 fields required**: No partial themes

Subagents: Document any additional design decisions made during implementation.
