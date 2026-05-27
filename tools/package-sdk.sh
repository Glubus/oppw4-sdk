#!/usr/bin/env bash
set -euo pipefail

TARGET="${TARGET:-x86_64-pc-windows-msvc}"
PROFILE="${PROFILE:-release}"
OUT_DIR="${OUT_DIR:-dist/oppw4-sdk}"
SKIP_BUILD="${SKIP_BUILD:-0}"
INCLUDE_LOADER="${INCLUDE_LOADER:-1}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOADER_ROOT="${LOADER_ROOT:-"$ROOT/../oppw4-modloader"}"
OUT_ROOT="$ROOT/$OUT_DIR"
PLUGINS_ROOT="$OUT_ROOT/plugins"
SDK_ROOT="$PLUGINS_ROOT/sdk"
TARGET_DIR="$ROOT/target/$TARGET/$PROFILE"
LOADER_TARGET_DIR="$LOADER_ROOT/target/$TARGET/$PROFILE"
DATA_ROOT="$ROOT/oppw4-data"

SDK_PACKAGES=(
  oppw4-sdk-core-plugin
  oppw4-sdk-data-plugin
  oppw4-sdk-runtime-plugin
  oppw4-sdk-debug-plugin
  oppw4-sdk-overlay-plugin
  oppw4-sdk-linkdata-plugin
  oppw4-sdk-rdb-plugin
)
OFFICIAL_PACKAGES=(
  oppw4-moveset-patcher-plugin
)

copy_required_file() {
  local source="$1"
  local destination="$2"
  if [[ ! -f "$source" ]]; then
    echo "missing required file: $source" >&2
    exit 1
  fi
  mkdir -p "$(dirname "$destination")"
  cp -f "$source" "$destination"
}

copy_required_dir() {
  local source="$1"
  local destination="$2"
  if [[ ! -d "$source" ]]; then
    echo "missing required directory: $source" >&2
    echo "run: git submodule update --init --recursive" >&2
    exit 1
  fi
  mkdir -p "$(dirname "$destination")"
  cp -R "$source" "$destination"
}

if [[ "$SKIP_BUILD" != "1" ]]; then
  release_args=()
  if [[ "$PROFILE" == "release" ]]; then
    release_args+=(--release)
  fi
  if [[ "$INCLUDE_LOADER" == "1" ]]; then
    if [[ ! -f "$LOADER_ROOT/Cargo.toml" ]]; then
      echo "missing loader workspace: $LOADER_ROOT" >&2
      exit 1
    fi
    cargo build --manifest-path "$LOADER_ROOT/Cargo.toml" -p oppw4-dinput8-proxy --target "$TARGET" "${release_args[@]}"
  fi
  for package in "${SDK_PACKAGES[@]}" "${OFFICIAL_PACKAGES[@]}"; do
    cargo build -p "$package" --target "$TARGET" "${release_args[@]}"
  done
fi

rm -rf "$OUT_ROOT"
mkdir -p "$SDK_ROOT"

if [[ "$INCLUDE_LOADER" == "1" ]]; then
  copy_required_file "$LOADER_TARGET_DIR/dinput8.dll" "$OUT_ROOT/dinput8.dll"
fi
copy_required_file "$TARGET_DIR/sdk.dll" "$SDK_ROOT/sdk.dll"
copy_required_file "$TARGET_DIR/data.dll" "$SDK_ROOT/data.dll"
copy_required_file "$TARGET_DIR/runtime.dll" "$SDK_ROOT/runtime.dll"
copy_required_file "$TARGET_DIR/debug.dll" "$SDK_ROOT/debug.dll"
copy_required_file "$TARGET_DIR/overlay.dll" "$SDK_ROOT/overlay.dll"
copy_required_file "$TARGET_DIR/linkdata.dll" "$SDK_ROOT/linkdata.dll"
copy_required_file "$TARGET_DIR/rdb.dll" "$SDK_ROOT/rdb.dll"
copy_required_file "$ROOT/sdk/plugins/core/plugin.toml" "$SDK_ROOT/plugin.toml"

for plugin in moveset_patcher; do
  plugin_root="$PLUGINS_ROOT/$plugin"
  source_root="$ROOT/plugins/$plugin"
  mkdir -p "$plugin_root"
  copy_required_file "$TARGET_DIR/$plugin.dll" "$plugin_root/$plugin.dll"
  copy_required_file "$source_root/plugin.toml" "$plugin_root/plugin.toml"
done

mkdir -p "$OUT_ROOT/oppw4-data"
copy_required_file "$DATA_ROOT/README.md" "$OUT_ROOT/oppw4-data/README.md"
copy_required_dir "$DATA_ROOT/characters" "$OUT_ROOT/oppw4-data/characters"
copy_required_dir "$DATA_ROOT/generated" "$OUT_ROOT/oppw4-data/generated"
copy_required_dir "$DATA_ROOT/missions" "$OUT_ROOT/oppw4-data/missions"
copy_required_dir "$DATA_ROOT/schemas" "$OUT_ROOT/oppw4-data/schemas"
mkdir -p "$OUT_ROOT/mods"
copy_required_dir "$ROOT/examples/js" "$OUT_ROOT/examples/js"
copy_required_dir "$ROOT/examples/rust/log_plugin" "$OUT_ROOT/examples/rust/log_plugin"
copy_required_dir "$ROOT/examples/rust/native_mod" "$OUT_ROOT/examples/rust/native_mod"

echo "SDK package written to $OUT_ROOT"
