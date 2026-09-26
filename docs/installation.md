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

Downgrades are not supported: the older build refuses to start when `.schema_version` records a newer data schema, and that refusal is the safe outcome, because a release that retypes a persisted field writes a shape the previous one cannot read.

### Finding out whether an older build can read your data

1. Copy every `profiles/<name>/sessions.json` and the `groups.json` beside it out of the [app directory](guides/configuration.md#file-locations), plus the top-level `sessions.json` if the pre-profiles layout left one. A retype migration rewrites no other file; `groups.json` comes along so a return trip restores a matching pair.
2. Copy `.schema_version` as a record, then edit it in place down to the number the older build prints when it refuses to start, its own schema version. Deleting it is not the safer variant: a missing file reads as version 0, so the older build re-runs every migration it has.
3. Move any `sessions.corrupt.jsonl` or `groups.corrupt.jsonl` aside rather than deleting it, so step 4 can tell a new one from an old one.
4. Start the older build once, then look for either of those files. It writes one as soon as it reads a row it cannot decode. Create, rename, and archive nothing while it runs: its next save writes back only the rows it read, and the rest are gone from the file.
5. If either file appeared, put your copies back and stay on the current release.

### When an older build could not read your sessions

The quarantine file is then the only copy of those rows, and it holds one load's worth of them: that load writes the whole batch it could not decode, and a later load replaces the file, so whatever that later load no longer saw is gone from it.

A release that retypes a persisted field also leaves a `sessions.json.pre-recovery-<stamp>` beside the file, holding the bytes from just before that rewrite. The stamp is a clock reading pushed above its siblings, so order the names rather than reading them as dates. When one upgrade retypes more than one field each rewrite leaves its own, and only the oldest is still a shape the older build reads, so restore that one. Three are kept per file, shared with the copies `aoe` takes when it repairs a group move, so two repairs can push the oldest out, and then none of the rest will read either.

Check that the copy is there rather than assuming it: an upgrade that could not take one says so in its startup output and in the log. These retypes shipped in 1.17.0, so an install that already ran them has no copy of the older shape, and one only helps an install still on 1.16.x that upgrades into a build carrying them. Using it means editing `.schema_version` as in step 2.

## Uninstalling

```bash
aoe uninstall
```

It prompts before removing the binary, the app data directory, and the tmux settings.
