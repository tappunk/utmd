[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Crates.io Version](https://img.shields.io/crates/v/utmd?color=orange&cacheSeconds=3600)](https://crates.io/crates/utmd)
[![GitHub Release](https://img.shields.io/github/v/release/tappunk/utmd?color=blue)](https://github.com/tappunk/utmd/releases)
[![X Follow](https://img.shields.io/twitter/follow/tappunk?style=social)](https://x.com/tappunk)

# utmd

Disposable VM sandbox manager for UTM on macOS.

Clone base templates into isolated development environments prefixed with `utmd-`. Delete all sandboxes with a single command while leaving personal VMs untouched.

## Prerequisites

- macOS with [UTM Desktop Application](https://mac.getutm.app/) installed
- Base VM templates named `[t]-linux` and `[t]-macos` in UTM

## Usage

```bash
utmd clone linux              # Clone from [t]-linux (name: utmd-linux-<hash>)
utmd clone macos              # Clone from [t]-macos (name: utmd-macos-<hash>)
utmd clone linux sandbox1     # Clone with custom name (becomes utmd-sandbox1)

utmd delete-all               # Delete all VMs prefixed with "utmd-"
```

List all VMs in UTM:

```bash
utmctl list
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

On first run, `utmd` checks for `utmctl` and offers to create a symlink if it is not found.
