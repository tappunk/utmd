![utmd](https://raw.githubusercontent.com/tappunk/.github/refs/heads/main/assets/utmd.webp)

[![License: MIT](https://img.shields.io/badge/License-MIT-orange.svg)](LICENSE)
[![Crates.io Version](https://img.shields.io/crates/v/utmd?color=orange&cacheSeconds=3600)](https://crates.io/crates/utmd)
[![GitHub Release](https://img.shields.io/github/v/release/tappunk/utmd?color=orange)](https://github.com/tappunk/utmd/releases)
[![X Follow](https://img.shields.io/twitter/follow/tappunk?style=social)](https://x.com/tappunk)

# utmd

**Disposable VM sandbox manager for UTM on macOS.** Create, run, and prune isolated development environments. A personal tool for my own workflow, not a product. Expect breaking changes and best-effort maintenance.

[Installation](#installation) • [Prerequisites](#prerequisites) • [Quick Start](#quick-start) • [Usage](#usage) • [Config](#config) • [JSON Output](#json-output)

## Features

- **Template-based cloning**: create VMs by cloning base templates (`[t]-linux`, `[t]-macos`)
- **Disposable lifecycle**: one-off sandboxes via `create` → `run` → `rm`, batch cleanup via `prune`
- **Flexible naming**: default template `{prefix}{os}-{rand}`, custom templates using `{prefix}`, `{os}`, `{date}`, `{time}`, `{rand}`, or exact names with `--name`
- **Batch pruning**: filter by prefix, OS, or age (`--older-than 24h`)
- **JSON output**: every command emits the same wrapper shape, ready for scripting
- **Safe by default**: `prune` only touches `utmd-` prefixed VMs unless you pass a different `--prefix`
- **Dry run**: `--dry-run` previews actions without touching state
- **Global flags**: `--json`, `--quiet`, `--yes`, `--dry-run`, `--config`

## Prerequisites

- macOS with **UTM Desktop** installed (default `/Applications/UTM.app`). `utmd` drives UTM through `utmctl`, the CLI embedded in the app. No separate `utmctl` install needed.
- A **GUI login session**. `utmctl` drives the UTM app via AppleScript, which needs a GUI session, so it fails over SSH, in cron, or before a user logs in.

`utmd` resolves `utmctl` in this order (first hit wins):

1. Explicit `utmctl_path` in config or `UTMD_UTMCTL_PATH`
2. `utmctl` on `PATH`
3. Fallbacks: `/Applications/UTM.app/Contents/MacOS/utmctl`, `/opt/homebrew/bin/utmctl`, `/usr/local/bin/utmctl`, `~/Applications/UTM.app/Contents/MacOS/utmctl`

utmd canonicalizes the resolved path (symlinks followed) and rejects it unless it ends with `UTM.app/Contents/MacOS/utmctl`, so the app-embedded binary is the only one it will run.

## Installation

### Homebrew

```bash
brew tap tappunk/tap
brew trust tappunk/tap
brew install tappunk/tap/utmd
```

### Cargo

```bash
cargo install utmd
```

### Build from source

```bash
git clone https://github.com/tappunk/utmd.git
cd utmd
cargo build --release
sudo cp target/release/utmd /usr/local/bin/utmd
```

## Quick Start

```bash
utmd init                    # Create config file
utmd create linux            # Clone a sandbox from template
utmd run linux               # Boot and show the sandbox
```

## Usage

### Create and run sandboxes

```bash
utmd create linux                      # Clone from template, name generated
utmd create linux --name sandbox1      # Clone with a specific name
utmd create linux --name exact-name --name-exact
utmd create linux --name-template "{prefix}{os}-{rand}"

utmd run linux                   # Clone and run in one step
utmd run linux --name myproject  # Clone, run, and show
utmd run linux --name-template "{prefix}{os}-{rand}"
```

### Manage existing VMs

```bash
utmd ls                              # List managed VMs (default prefix)
utmd ls --prefix ""                  # List all VMs
utmd inspect utmd-linux-abc123       # Show VM details
utmd start utmd-linux-abc123         # Start a stopped VM
utmd stop utmd-linux-abc123          # Stop a running VM
utmd show utmd-linux-abc123          # Open in UTM app
utmd rm utmd-linux-abc123            # Remove a single VM
```

### Batch pruning

```bash
utmd prune                             # Prune all disposable VMs
utmd prune --prefix utmd-              # Prune with specific prefix
utmd prune --os linux                  # Prune only Linux VMs
utmd prune --older-than 24h            # Prune VMs older than 24 hours
utmd prune --older-than 7d --dry-run   # Preview what would be deleted
utmd --yes prune                       # Skip confirmation prompts
```

## Config

`utmd init` writes a boilerplate config to `~/.config/utmd/config.toml` (override the location with `--config`):

```toml
utm_app = "/Applications/UTM.app"
#utmctl_path = "/Applications/UTM.app/Contents/MacOS/utmctl"
state_path = "/Users/user/.config/utmd/state.json"
default_prefix = "utmd-"

[templates]
linux = "[t]-linux"
macos = "[t]-macos"

[naming]
default_template = "{prefix}{os}-{rand}"
rand_len = 4
max_retries = 8

[output]
default_json = false
default_quiet = false
```

### Environment variables

Precedence: **CLI flags > environment > config file > built-in defaults**.

```bash
UTMD_UTM_APP
UTMD_UTMCTL_PATH
UTMD_STATE_PATH
UTMD_PREFIX
UTMD_TEMPLATE_LINUX
UTMD_TEMPLATE_MACOS
UTMD_JSON
UTMD_QUIET
```

## JSON Output

All commands return wrapped JSON with a stable top-level shape:

```json
{
  "command": "ls",
  "ok": true,
  "data": [],
  "warnings": [],
  "error": null
}
```

Use `--json` to force JSON output. Before any command runs, `utmd` resolves `utmctl` (see [Prerequisites](#prerequisites)) and exits with a diagnostic if it cannot find it.
