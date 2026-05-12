#!/usr/bin/env bash
set -euo pipefail

bench_filter="${1:-tip_to_tip_fallback_storm/4096}"
sample_size="${CARGO_BENCH_SAMPLE_SIZE:-20}"
measurement_time="${CARGO_BENCH_MEASUREMENT_TIME:-2}"
warm_up_time="${CARGO_BENCH_WARM_UP_TIME:-1}"

bench_cmd=(
  cargo bench
  --bench tip_to_tip
  --
  --sample-size "$sample_size"
  --measurement-time "$measurement_time"
  --warm-up-time "$warm_up_time"
  "$bench_filter"
)

if command -v perf >/dev/null 2>&1; then
  set +e
  perf stat -d -- "${bench_cmd[@]}"
  status="$?"
  set -e

  if [ "$status" -eq 0 ]; then
    exit 0
  fi

  echo "perf stat was unavailable or denied; falling back to Criterion only." >&2
fi

"${bench_cmd[@]}"
