#!/bin/sh

set -eu

if [ -z "${PLUGIN_ROOT:-}" ]; then
  script_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
  PLUGIN_ROOT=$(dirname -- "$script_directory")
fi

case "$(uname -sm)" in
  "Darwin arm64")
    target="aarch64-apple-darwin"
    ;;
  "Darwin x86_64")
    target="x86_64-apple-darwin"
    ;;
  "Linux aarch64"|"Linux arm64")
    target="aarch64-unknown-linux-musl"
    ;;
  "Linux x86_64"|"Linux amd64")
    target="x86_64-unknown-linux-musl"
    ;;
  *)
    echo "codex-context-window: unsupported platform: $(uname -sm)" >&2
    exit 1
    ;;
esac

binary="${PLUGIN_ROOT}/bin/${target}/codex-context-window"
if [ ! -x "$binary" ]; then
  target_binary="${PLUGIN_ROOT}/target/${target}/release/codex-context-window"
  local_binary="${PLUGIN_ROOT}/target/release/codex-context-window"

  if [ -x "$target_binary" ]; then
    binary="$target_binary"
  elif [ -x "$local_binary" ]; then
    binary="$local_binary"
  else
    echo "codex-context-window: native binary is missing for ${target}" >&2
    exit 1
  fi
fi

exec "$binary"
