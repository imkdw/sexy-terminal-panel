#!/usr/bin/env bash
set -euo pipefail

PREFIX="/usr/local"
while [ "$#" -gt 0 ]; do
  case "$1" in
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

rm -f "$PREFIX/bin/stp"
printf 'removed %s/bin/stp\n' "$PREFIX"
printf 'manual state cleanup: rm -rf "${XDG_STATE_HOME:-$HOME/.local/state}/sexy-terminal-panel"\n'
