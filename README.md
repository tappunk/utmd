![utmd](https://raw.githubusercontent.com/tappunk/.github/refs/heads/main/assets/utmd.webp)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Crates.io Version](https://img.shields.io/crates/v/utmd?color=orange&cacheSeconds=3600)](https://crates.io/crates/utmd)
[![GitHub Release](https://img.shields.io/github/v/release/tappunk/utmd?color=blue)](https://github.com/tappunk/utmd/releases)
[![X Follow](https://img.shields.io/twitter/follow/tappunk?style=social)](https://x.com/tappunk)

# utmd

**Disposable VM sandbox manager for UTM on macOS.** Create, run, and prune isolated development environments.

[Installation](#installation) • [Quick Start](#quick-start) • [Usage](#usage) • [Config](#config) • [JSON Output](#json-output)

## Features

- **Template-based cloning** — create VMs from base templates (`[t]-linux`, `[t]-macos`)
- **Disposable lifecycle** — `create` → `run` → `rm` for one-off sandboxes, `prune` for batch cleanup
- **Smart naming** — exact names, templates with `{prefix}{os}-{rand}`, or prefix + OS combinations
- **Batch pruning** — filter by prefix, OS, or age (`--older-than 24h`, `--older-than 7d`)
- **Machine readable** — JSON output for all commands, designed for automation and agent pipelines
- **Non-destructive** — only removes `utmd-` prefixed VMs, leaves personal VMs untouched
- **Dry run support** — `--dry-run` previews actions without mutating state
- **Global automation flags** — `--json`, `--quiet`, `--yes`, `--dry-run`, `--config`

## Installation

### Homebrew

```bash
brew install tappunk/utmd/utmd
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

Create the config file with `utmd init`:

```bash
utmd init
```

Default config path: `~/.config/utmd/config.toml`

```toml
utm_app = "/Applications/UTM.app"
utmctl_path = "/usr/local/bin/utmctl"
state_path = "/Users/user/Library/Application Support/utmd/state.json"
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

Environment variables override config values. Precedence: **CLI flags > environment > config file > built-in defaults**.

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

Use `--json` to force JSON output. On first run, `utmd` checks for the `utmctl` dependency and reports an error if it is missing.
