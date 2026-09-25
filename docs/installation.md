# Installation

**Prerequisites:** [tmux](https://github.com/tmux/tmux/wiki). Docker (or another container runtime) is optional, for [sandboxing](guides/sandbox.md), and [Node.js](https://nodejs.org/) is needed only to build the web dashboard from source. Building also needs a C toolchain for the bundled native dependencies (SQLite, libgit2, OpenSSL, liblzma, AWS-LC); a stock `cc` covers most platforms, and targets without pre-generated AWS-LC bindings also need CMake.

## Install

```bash
# Quick install (Linux and macOS)
curl -fsSL \
  https://raw.githubusercontent.com/agent-of-empires/agent-of-empires/main/scripts/install.sh \
  | bash

# Homebrew
brew install aoe

# From source; add --features web for the dashboard (needs Node and npm)
git clone https://github.com/agent-of-empires/agent-of-empires
cd agent-of-empires && cargo build --release
```

A source build leaves the binary at `target/release/aoe`. Verify any install with `aoe --version`.

## Updating

```bash
aoe update
```

`aoe update` detects how aoe was installed (Homebrew, the install script, Nix, or Cargo) and dispatches to the right mechanism. For Nix and Cargo it prints the manual command instead, since those need external tooling. In the TUI, press `u` while the update bar is visible to run the same flow, or `Ctrl+x` to dismiss the bar.

If you installed shell completions as a static file, regenerate it afterwards so it picks up new commands and flags; see [Shell completions](guides/shell-completions.md) for the always-fresh setup that avoids this.

## Downgrading

Downgrades are not supported. The older build refuses to start when `.schema_version` records a newer data schema, which is the safe outcome. A release that retypes a persisted field writes a shape the previous release cannot read, and that release moves any row it cannot decode to a sidecar beside the file, `sessions.corrupt.jsonl` for sessions and `groups.corrupt.jsonl` for groups, then rewrites the file without it on its next save. The row itself survives in that sidecar, so nothing is erased; what is lost is the older build's ability to use the session without hand-repairing the file.

If you need to go back anyway, first copy `profiles/<name>/sessions.json`, `profiles/<name>/groups.json`, and `.schema_version` out of the [app directory](guides/configuration.md#file-locations), and delete any `sessions.corrupt.jsonl` or `groups.corrupt.jsonl` left by an earlier problem so their presence later means something. There is no flag to bypass the check: editing `.schema_version` down is the only way in. Launch the older build once without creating, renaming, or archiving any session, then check whether it wrote either sidecar. If it did, put your copies back and return to the current release. Deleting `.schema_version` instead of editing it is not a safer variant: the older build reads that as a fresh install and re-runs every migration it has.

A release that retypes a persisted field also leaves a `sessions.json.pre-recovery-<millis>` restore point beside the file, holding the bytes as they were just before that rewrite. When one upgrade retypes more than one field each rewrite leaves its own, and only the oldest of that run is still in a shape the previous release reads, so restore the oldest one still there. Three are kept per file, shared with the backups `aoe` takes when it repairs a group move, so a repair after the upgrade can push the oldest one out; if that happened, none of the rest is safe and copying the file is the only path. The retypes this covers shipped in 1.17.0, so an install that has already run them has no restore point for the 1.16 shape; it helps only an install still on 1.16.x that upgrades into a build carrying it. Using it still means editing `.schema_version` as above.

## Uninstalling

```bash
aoe uninstall
```

It prompts before removing the binary, the app data directory, and the tmux settings.
