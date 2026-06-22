![utmd](https://raw.githubusercontent.com/tappunk/.github/refs/heads/main/assets/utmd.webp)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Crates.io Version](https://img.shields.io/crates/v/utmd?color=orange&cacheSeconds=3600)](https://crates.io/crates/utmd)
[![GitHub Release](https://img.shields.io/github/v/release/tappunk/utmd?color=blue)](https://github.com/tappunk/utmd/releases)
[![X Follow](https://img.shields.io/twitter/follow/tappunk?style=social)](https://x.com/tappunk)

# utmd

Disposable VM sandbox manager for UTM on macOS.

Clone base templates into isolated development environments prefixed with `utmd-`. Delete all sandboxes with a single command while leaving personal VMs untouched.

Global flags for automation and scripting:

```bash
--json      # machine-readable command response
--quiet     # suppress info logs in human mode
--yes       # skip confirmations for destructive actions
--dry-run   # show actions without mutating state
--config    # custom config file path
```

## Prerequisites

- macOS with [UTM Desktop Application](https://mac.getutm.app/) installed
- Base VM templates named `[t]-linux` and `[t]-macos` in UTM

## Usage

```bash
utmd clone linux
utmd clone macos
utmd clone linux --name sandbox1
utmd clone linux --name exact-name --name-exact
utmd clone linux --name-template "{prefix}{os}-{date}-{rand}"

utmd list                              # lists VMs using configured default_prefix
utmd list --prefix ""                  # list all VMs
utmd status utmd-linux-abc123
utmd start utmd-linux-abc123
utmd stop utmd-linux-abc123
utmd open utmd-linux-abc123
utmd delete utmd-linux-abc123

utmd delete-all
utmd delete-all --prefix utmd- --os linux --older-than 24h --dry-run
utmd --yes delete-all --prefix utmd-
```

`delete-all --older-than` currently supports `h` (hours) and `d` (days), for example `24h` and `7d`.

## Config

Default config path:

```bash
~/.config/utmd/config.toml
```

Example:

```toml
utm_app = "/Applications/UTM.app"
utmctl_path = "/usr/local/bin/utmctl"
state_path = "/Users/you/Library/Application Support/utmd/state.json"
default_prefix = "utmd-"

[templates]
linux = "[t]-linux"
macos = "[t]-macos"

[naming]
default_template = "{prefix}{os}-{date}-{rand}"
rand_len = 6
max_retries = 8

[output]
default_json = false
default_quiet = false
```

Environment overrides:

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

Precedence:

```bash
CLI > environment > config file > built-in defaults
```

## JSON output

All commands return wrapped JSON with a stable top-level shape:

```json
{
  "command": "list",
  "ok": true,
  "data": [],
  "warnings": [],
  "error": null
}
```

Exit codes:

```bash
0  success
2  invalid usage or validation
3  dependency missing
4  not found
5  conflict
6  partial failure
10 external command failure
```

## Installation

utmd is available on [crates.io](https://crates.io/crates/utmd) and [Homebrew](https://brew.sh/).

### Cargo

```bash
cargo install utmd
```

### Homebrew

```bash
brew install tappunk/utmd/utmd
```

### Build from Source

```bash
git clone https://github.com/tappunk/utmd.git
cd utmd
cargo build --release
sudo cp target/release/utmd /usr/local/bin/utmd
```

On first run, `utmd` checks for `utmctl` and reports a dependency error when it is unavailable.

## Local Verification

Run the local verification gate before releases:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```
