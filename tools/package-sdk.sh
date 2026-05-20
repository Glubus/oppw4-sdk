#!/usr/bin/env bash
set -euo pipefail

TARGET="${TARGET:-x86_64-pc-windows-gnu}"
PROFILE="${PROFILE:-release}"
OUT_DIR="${OUT_DIR:-dist/oppw4-sdk}"
SKIP_BUILD="${SKIP_BUILD:-0}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_ROOT="$ROOT/$OUT_DIR"
PLUGINS_ROOT="$OUT_ROOT/plugins"
SDK_ROOT="$PLUGINS_ROOT/sdk"
TARGET_DIR="$ROOT/target/$TARGET/$PROFILE"

SDK_PACKAGES=(
  oppw4-sdk-core-plugin
  oppw4-sdk-runtime-plugin
  oppw4-sdk-linkdata-plugin
  oppw4-sdk-rdb-plugin
)
OFFICIAL_PACKAGES=(
  oppw4-skin-patcher-plugin
  oppw4-fx-director-plugin
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

if [[ "$SKIP_BUILD" != "1" ]]; then
  release_args=()
  if [[ "$PROFILE" == "release" ]]; then
    release_args+=(--release)
  fi
  for package in "${SDK_PACKAGES[@]}" "${OFFICIAL_PACKAGES[@]}"; do
    cargo build -p "$package" --target "$TARGET" "${release_args[@]}"
  done
fi

rm -rf "$OUT_ROOT"
mkdir -p "$SDK_ROOT"

copy_required_file "$TARGET_DIR/sdk.dll" "$SDK_ROOT/sdk.dll"
copy_required_file "$TARGET_DIR/runtime.dll" "$SDK_ROOT/runtime.dll"
copy_required_file "$TARGET_DIR/linkdata.dll" "$SDK_ROOT/linkdata.dll"
copy_required_file "$TARGET_DIR/rdb.dll" "$SDK_ROOT/rdb.dll"

for plugin in skin_patcher fx_director moveset_patcher; do
  plugin_root="$PLUGINS_ROOT/$plugin"
  source_root="$ROOT/official_plugins/$plugin"
  mkdir -p "$plugin_root"
  copy_required_file "$TARGET_DIR/$plugin.dll" "$plugin_root/$plugin.dll"
  copy_required_file "$source_root/plugin.toml" "$plugin_root/plugin.toml"
done

mkdir -p "$OUT_ROOT/mods"

echo "SDK package written to $OUT_ROOT"
