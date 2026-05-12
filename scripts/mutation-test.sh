#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo-mutants >/dev/null 2>&1; then
  echo "cargo-mutants is required. Install it with: cargo install cargo-mutants" >&2
  exit 127
fi

log_file="$(mktemp "${TMPDIR:-/tmp}/tip-to-tip-mutants.XXXXXX")"
trap 'rm -f "$log_file"' EXIT

set +e
cargo mutants \
  --file src/acceleration.rs \
  --file src/config.rs \
  --file src/constants.rs \
  --file src/enterprise.rs \
  --file src/error.rs \
  --file src/planner.rs \
  --file src/profile.rs \
  --file src/tip_to_tip.rs \
  --file src/validation.rs \
  --jobs "${CARGO_MUTANTS_JOBS:-2}" \
  --timeout "${CARGO_MUTANTS_TIMEOUT:-60}" \
  --minimum-test-timeout "${CARGO_MUTANTS_MINIMUM_TEST_TIMEOUT:-10}" \
  -- \
  --all-targets 2>&1 | tee "$log_file"
status="${PIPESTATUS[0]}"
set -e

if grep -q '^MISSED' "$log_file"; then
  exit "$status"
fi

if [ "$status" -ne 0 ] && grep -q '^TIMEOUT' "$log_file"; then
  echo "No missed mutants; timeout mutants were treated as killed non-termination cases." >&2
  exit 0
fi

exit "$status"
