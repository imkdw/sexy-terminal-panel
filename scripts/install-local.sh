#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PREFIX="/usr/local"
DRY_RUN=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --prefix)
      PREFIX="$2"
      shift 2
      ;;
    *)
      printf 'unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
done

if ! command -v code >/dev/null 2>&1; then
  printf 'code CLI not found\n' >&2
  exit 1
fi

printf 'build binary: cargo build --release -p stp\n'
printf 'install binary: %s/bin/stp\n' "$PREFIX"
printf 'package VSIX: cd extensions/vscode && bun run package-vsix\n'
printf 'VS Code install command: code --install-extension sexy-terminal-panel-vscode-0.1.0.vsix\n'

if [ "$DRY_RUN" -eq 1 ]; then
  exit 0
fi

cd "$ROOT"
cargo build --release -p stp
mkdir -p "$PREFIX/bin"
TMP_BIN="$(mktemp "$PREFIX/bin/stp.XXXXXX")"
cp target/release/stp "$TMP_BIN"
chmod 755 "$TMP_BIN"
mv -f "$TMP_BIN" "$PREFIX/bin/stp"
(cd extensions/vscode && bun run package-vsix)
code --install-extension "$ROOT/extensions/vscode/sexy-terminal-panel-vscode-0.1.0.vsix"
