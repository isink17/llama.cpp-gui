#!/usr/bin/env bash
set -euo pipefail

skip_prereq_check=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-prereq-check)
      skip_prereq_check=true
      shift
      ;;
    -h|--help)
      cat <<'EOF'
Usage: Invoke-SmokeChecks.sh [--skip-prereq-check]

Runs the fast smoke checks for the migration slice.
EOF
      exit 0
      ;;
    *)
      printf 'Unknown argument: %s\n' "$1" >&2
      exit 1
      ;;
  esac
done

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
ui_dir="$repo_root/ui"
tauri_dir="$repo_root/src-tauri"
cargo_toml="$tauri_dir/Cargo.toml"

test_smoke_prereqs() {
  local missing=()

  for command_name in cargo npm; do
    if ! command -v "$command_name" >/dev/null 2>&1; then
      missing+=("$command_name")
    fi
  done

  if (( ${#missing[@]} > 0 )); then
    printf 'Missing required commands: %s\n' "${missing[*]}" >&2
    exit 1
  fi

  printf 'Smoke prereqs available: cargo, npm\n'
}

invoke_checked_command() {
  local working_directory="$1"
  shift

  printf 'Running:'
  printf ' %q' "$@"
  printf '\n'

  (
    cd "$working_directory"
    "$@"
  )
}

if [[ "$skip_prereq_check" == false ]]; then
  test_smoke_prereqs
fi

if [[ ! -d "$ui_dir" ]]; then
  printf 'Missing UI directory: %s\n' "$ui_dir" >&2
  exit 1
fi

if [[ ! -f "$cargo_toml" ]]; then
  printf 'Missing Cargo manifest: %s\n' "$cargo_toml" >&2
  exit 1
fi

invoke_checked_command "$ui_dir" npm run build
invoke_checked_command "$tauri_dir" cargo fmt --check --manifest-path "$cargo_toml"

printf 'Smoke checks completed successfully.\n'
