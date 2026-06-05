#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

status=0
while IFS= read -r file; do
  lines="$(awk '!/^[[:space:]]*$/ && !/^[[:space:]]*(\/\/|#)/ { count++ } END { print count + 0 }' "$file")"
  if [ "$lines" -gt 250 ]; then
    printf 'LOC FAIL %s %s\n' "$file" "$lines"
    status=1
  fi
done < <(find crates extensions/vscode/src extensions/vscode/tests -type f \( -name '*.rs' -o -name '*.ts' \) | sort)

if [ "$status" -ne 0 ]; then
  exit "$status"
fi

printf 'LOC PASS\n'
