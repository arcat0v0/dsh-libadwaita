#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
  BLUE=$'\033[1;34m'
  GREEN=$'\033[1;32m'
  RED=$'\033[1;31m'
  BOLD=$'\033[1m'
  RESET=$'\033[0m'
else
  BLUE=''
  GREEN=''
  RED=''
  BOLD=''
  RESET=''
fi

checks=0
failures=()

section() {
  printf '\n%s==> %s%s\n' "$BLUE" "$1" "$RESET"
}

run() {
  local label="$1"
  shift
  section "$label"
  checks=$((checks + 1))
  if "$@"; then
    printf '%sPASS%s %s\n' "$GREEN" "$RESET" "$label"
  else
    printf '%sFAIL%s %s\n' "$RED" "$RESET" "$label"
    failures+=("$label")
  fi
}

printf '%s%s%s\n' "$BOLD" "dsh-libadwaita ci" "$RESET"
printf 'repository: %s\n' "$ROOT_DIR"

run "bridge: typecheck" pnpm -C bridge typecheck
run "bridge: build" pnpm -C bridge build
run "bridge: test" pnpm -C bridge test
run "bridge: lint" pnpm -C bridge lint
run "app: cargo check" cargo check --manifest-path app/Cargo.toml
run "app: cargo test" cargo test --manifest-path app/Cargo.toml
run "packaging: desktop file" desktop-file-validate packaging/io.github.deepseek_ai.DshLibadwaita.desktop

section "summary"
if (( ${#failures[@]} == 0 )); then
  printf '%sall %d checks passed%s\n' "$GREEN" "$checks" "$RESET"
  exit 0
fi

printf '%s%d of %d checks failed%s\n' "$RED" "${#failures[@]}" "$checks" "$RESET"
for failure in "${failures[@]}"; do
  printf '  %s- %s%s\n' "$RED" "$failure" "$RESET"
done
exit 1
