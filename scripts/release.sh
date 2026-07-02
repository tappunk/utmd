#!/bin/bash
set -euo pipefail

BIN_NAME="utmd"
TARGET_ARCH="macos-arm64"
RUST_TARGET="aarch64-apple-darwin"
GITHUB_REPO="tappunk/utmd"

DRY_RUN=false
if [[ "${1:-}" == "--dry-run" ]]; then
  DRY_RUN=true
  shift
fi

BUMP="${1:-patch}"
NOTES="${2:-}"

echo "[PROC] Verifying deployment dependencies..."
for tool in cargo gh shasum tar awk git sed; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "[ERR] Required CLI tool '$tool' is missing."
    exit 1
  fi
done

if ! $DRY_RUN; then
  if ! gh auth status >/dev/null 2>&1; then
    echo "[ERR] GitHub CLI is not authenticated. Run 'gh auth login'."
    exit 1
  fi
fi

if [[ ! "$BUMP" =~ ^(patch|minor|major)$ ]]; then
  echo "[ERR] Invalid bump type '$BUMP'. Use: patch, minor, or major"
  exit 1
fi

if [[ -n $(git ls-files --others --exclude-standard) ]]; then
  echo "[ERR] Untracked files found in workspace. Add or remove them before releasing."
  git ls-files --others --exclude-standard
  exit 1
fi

if [[ -n $(git status --porcelain) ]]; then
  echo "[ERR] Uncommitted changes detected. Stash or commit before releasing."
  exit 1
fi

if [[ $(git branch --show-current) != "main" ]]; then
  echo "[ERR] You must be on the 'main' branch to cut a release."
  exit 1
fi

git fetch origin
LOCAL_HEAD=$(git rev-parse HEAD)
REMOTE_HEAD=$(git rev-parse origin/main)
if [[ "$LOCAL_HEAD" != "$REMOTE_HEAD" ]]; then
  echo "[ERR] Local 'main' must match 'origin/main' exactly. Pull/rebase/push until they match."
  exit 1
fi

echo "[PROC] Executing strict code quality gates..."
cargo fmt --check || {
  echo "[ERR] Code formatting violations found. Run 'cargo fmt'."
  exit 1
}
cargo clippy -- -D warnings || {
  echo "[ERR] Clippy warnings detected. Fix them before releasing."
  exit 1
}
cargo test || {
  echo "[ERR] Test suite execution failed."
  exit 1
}

CURRENT_VERSION=$(grep -m 1 '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
if [[ -z "$CURRENT_VERSION" ]]; then
  echo "[ERR] Could not read current version from Cargo.toml"
  exit 1
fi

IFS='.' read -r MAJOR MINOR PATCH <<<"$CURRENT_VERSION"
case "$BUMP" in
patch) PATCH=$((PATCH + 1)) ;;
minor)
  MINOR=$((MINOR + 1))
  PATCH=0
  ;;
major)
  MAJOR=$((MAJOR + 1))
  MINOR=0
  PATCH=0
  ;;
esac
NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"

echo "Preparing Apple Silicon Release: v$CURRENT_VERSION -> v$NEW_VERSION ($BUMP)"
if $DRY_RUN; then
  echo "[INFO] Dry run complete. Code is pristine and ready for release."
  exit 0
fi

if git rev-parse -q --verify "refs/tags/v$NEW_VERSION" >/dev/null 2>&1; then
  echo "[ERR] Tag 'v$NEW_VERSION' already exists locally."
  exit 1
fi
if git ls-remote --exit-code --tags origin "refs/tags/v$NEW_VERSION" >/dev/null 2>&1; then
  echo "[ERR] Tag 'v$NEW_VERSION' already exists on origin."
  exit 1
fi

BACKUP_CARGO_TOML=""
BACKUP_FLAKE_NIX=""
PUSHED=false
rollback() {
  echo ""
  echo "[CRIT] Release pipeline interrupted!"
  rm -f Cargo.toml.bak flake.nix.bak "${ARCHIVE_NAME:-}" "${CHECKSUM_NAME:-}" "$BACKUP_CARGO_TOML" "$BACKUP_FLAKE_NIX" 2>/dev/null || true
  if [[ -n "${STAGING_DIR:-}" && -d "${STAGING_DIR}" ]]; then
    rm -rf "${STAGING_DIR}"
  fi

  if $PUSHED; then
    echo "[WARN] Release was already pushed. Manual recovery may be required."
    return
  fi

  if git rev-parse "v$NEW_VERSION" >/dev/null 2>&1; then
    git tag -d "v$NEW_VERSION"
  fi
  if [[ -n "$BACKUP_CARGO_TOML" && -f "$BACKUP_CARGO_TOML" ]]; then
    cp "$BACKUP_CARGO_TOML" Cargo.toml
  fi
  if [[ -n "$BACKUP_FLAKE_NIX" && -f "$BACKUP_FLAKE_NIX" ]]; then
    cp "$BACKUP_FLAKE_NIX" flake.nix
  fi
  echo "[WARN] Rolled back local release artifacts. Re-run scripts/release.sh to try again."
}
trap rollback ERR

echo "[PROC] Updating versioning configuration..."
BACKUP_CARGO_TOML=$(mktemp)
BACKUP_FLAKE_NIX=$(mktemp)
cp Cargo.toml "$BACKUP_CARGO_TOML"
cp flake.nix "$BACKUP_FLAKE_NIX"

awk -v old="$CURRENT_VERSION" -v new="$NEW_VERSION" '
BEGIN { in_package = 0; replaced = 0 }
/^\[package\]$/ { in_package = 1; print; next }
/^\[[^]]+\]$/ { in_package = 0; print; next }
{
  if (in_package && !replaced && $0 == "version = \"" old "\"") {
    print "version = \"" new "\""
    replaced = 1
  } else {
    print
  }
}
END { if (!replaced) exit 1 }
' Cargo.toml > Cargo.toml.tmp
mv Cargo.toml.tmp Cargo.toml

sed -i.bak "s/version = \"$CURRENT_VERSION\";/version = \"$NEW_VERSION\";/" flake.nix
rm -f Cargo.toml.bak flake.nix.bak

cargo update -p "$BIN_NAME"

echo "[PROC] Compiling optimized release binary for Apple Silicon..."
cargo build --release --target "$RUST_TARGET"

echo "[PROC] Packaging distribution archives..."
ARCHIVE_NAME="${BIN_NAME}-${NEW_VERSION}-bin-${TARGET_ARCH}.tar.gz"
CHECKSUM_NAME="${ARCHIVE_NAME}.sha256"
STAGING_DIR="$(mktemp -d)"

mkdir -p "${STAGING_DIR}/${BIN_NAME}"
cp "target/${RUST_TARGET}/release/${BIN_NAME}" "${STAGING_DIR}/${BIN_NAME}/"
cp README.md LICENSE "${STAGING_DIR}/${BIN_NAME}/" 2>/dev/null || true

tar -czf "$ARCHIVE_NAME" -C "$STAGING_DIR" "${BIN_NAME}"
shasum -a 256 "$ARCHIVE_NAME" >"$CHECKSUM_NAME"
rm -rf "$STAGING_DIR"
rm -f "$BACKUP_CARGO_TOML" "$BACKUP_FLAKE_NIX"
BACKUP_CARGO_TOML=""
BACKUP_FLAKE_NIX=""

echo "[PROC] Recording version changes to Git history..."
git add Cargo.toml Cargo.lock flake.nix
git commit -m "chore: release v$NEW_VERSION [skip ci]"
git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"

echo "[PROC] Synchronizing changes with remote origin..."
git push origin main
git push origin "v$NEW_VERSION"
PUSHED=true

echo "[PROC] Waiting for GitHub mirror to expose tag v$NEW_VERSION..."
MAX_TAG_WAIT_ATTEMPTS=30
TAG_WAIT_INTERVAL_SECS=2
for ((attempt = 1; attempt <= MAX_TAG_WAIT_ATTEMPTS; attempt++)); do
  if gh api "repos/${GITHUB_REPO}/git/ref/tags/v${NEW_VERSION}" >/dev/null 2>&1; then
    echo "[PROC] GitHub mirror now contains tag v$NEW_VERSION"
    break
  fi

  if [[ "$attempt" -eq "$MAX_TAG_WAIT_ATTEMPTS" ]]; then
    echo "[ERR] Timed out waiting for GitHub mirror tag v$NEW_VERSION in ${GITHUB_REPO}"
    echo "[ERR] Confirm mirror health or push tag directly to GitHub, then re-run release."
    exit 1
  fi

  sleep "$TAG_WAIT_INTERVAL_SECS"
done

echo "[PROC] Deploying GitHub Release and assets..."
if [[ -n "$NOTES" ]]; then
  gh release create "v$NEW_VERSION" "$ARCHIVE_NAME" "$CHECKSUM_NAME" \
    -R "$GITHUB_REPO" \
    --title "v$NEW_VERSION" \
    --notes "$NOTES"
else
  gh release create "v$NEW_VERSION" "$ARCHIVE_NAME" "$CHECKSUM_NAME" \
    -R "$GITHUB_REPO" \
    --title "v$NEW_VERSION" \
    --generate-notes
fi

echo "[PROC] Propagating release configuration to Homebrew tap..."
RAW_SHA=$(awk '{print $1}' "${CHECKSUM_NAME}")
TAP_DIR="$(mktemp -d)"

git clone --depth 1 "git@github.com:tappunk/homebrew-utmd.git" "$TAP_DIR"

cat <<EOF >"${TAP_DIR}/Formula/utmd.rb"
class Utmd < Formula
  desc "Minimalist developer sandbox and disposable VM manager for UTM on macOS"
  homepage "https://github.com/tappunk/utmd"
  version "${NEW_VERSION}"

  depends_on arch: :arm64
  depends_on :macos

  url "https://github.com/tappunk/utmd/releases/download/v#{version}/utmd-#{version}-bin-macos-arm64.tar.gz"
  sha256 "${RAW_SHA}"

  def install
    bin.install "utmd"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/utmd --version")
  end
end
EOF

(
  cd "$TAP_DIR"
  git add Formula/utmd.rb
  git commit -m "bump: utmd v${NEW_VERSION}"
  git push origin main
)
rm -rf "$TAP_DIR"

echo "[PROC] Synchronizing local Homebrew tap mirror..."
LOCAL_TAP_DIR="${HOME}/src/homebrew-utmd"
if [[ ! -d "${LOCAL_TAP_DIR}/.git" ]]; then
  echo "[WARN] Skipping local tap sync. Repository not found at ${LOCAL_TAP_DIR}."
elif [[ -n $(git -C "${LOCAL_TAP_DIR}" status --porcelain) ]]; then
  echo "[WARN] Skipping local tap sync. Uncommitted changes in ${LOCAL_TAP_DIR}."
elif [[ $(git -C "${LOCAL_TAP_DIR}" branch --show-current) != "main" ]]; then
  echo "[WARN] Skipping local tap sync. ${LOCAL_TAP_DIR} is not on 'main'."
elif ! git -C "${LOCAL_TAP_DIR}" fetch origin >/dev/null 2>&1; then
  echo "[WARN] Skipping local tap sync. Failed to fetch origin in ${LOCAL_TAP_DIR}."
elif [[ -n $(git -C "${LOCAL_TAP_DIR}" log HEAD..origin/main --oneline) ]]; then
  if git -C "${LOCAL_TAP_DIR}" pull --ff-only >/dev/null 2>&1; then
    echo "[PROC] Local tap mirror synchronized: ${LOCAL_TAP_DIR}"
  else
    echo "[WARN] Skipping local tap sync. Fast-forward pull failed in ${LOCAL_TAP_DIR}."
  fi
else
  echo "[PROC] Local tap mirror already up to date: ${LOCAL_TAP_DIR}"
fi

echo "[PROC] Cleaning up local packaging assets..."
rm -f "$ARCHIVE_NAME" "$CHECKSUM_NAME"

echo "[PROC] Publishing crate package to crates.io..."
cargo publish

trap - ERR

echo "[ SUCCESS ] Release v$NEW_VERSION fully deployed!"
