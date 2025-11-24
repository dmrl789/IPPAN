#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

REPORT_DIR="$ROOT_DIR/benchmarks/reports"
mkdir -p "$REPORT_DIR"

STAMP="$(date -u +"%Y%m%dT%H%M%SZ")"
REPORT_FILE="$REPORT_DIR/benchmarks_${STAMP}.log"

echo "IPPAN benchmark run @ ${STAMP} (UTC)" | tee "$REPORT_FILE"
echo "Workspace: ${ROOT_DIR}" | tee -a "$REPORT_FILE"
echo "" | tee -a "$REPORT_FILE"

run_bench() {
    local crate="$1"
    local bench="$2"
    local label="$3"

    echo "==> ${label} (${crate}::${bench})" | tee -a "$REPORT_FILE"
    local tmp
    tmp="$(mktemp)"

    if cargo bench -p "${crate}" --bench "${bench}" -- --warm-up-time 0.5 --measurement-time 2 \
        2>&1 | tee "${tmp}"; then
        cat "${tmp}" >> "$REPORT_FILE"
        local time_line
        local thrpt_line
        time_line="$(grep -E 'time:' "${tmp}" | tail -n 1 | sed 's/^[[:space:]]*//')"
        thrpt_line="$(grep -E 'thrpt:' "${tmp}" | tail -n 1 | sed 's/^[[:space:]]*//')"
        if [[ -n "${time_line}" || -n "${thrpt_line}" ]]; then
            echo "   ${time_line}" | tee -a "$REPORT_FILE"
            echo "   ${thrpt_line}" | tee -a "$REPORT_FILE"
        fi
        echo "" | tee -a "$REPORT_FILE"
    else
        echo "   ❌ Benchmark failed – see report for details." | tee -a "$REPORT_FILE"
        cat "${tmp}" >> "$REPORT_FILE"
        rm -f "${tmp}"
        exit 1
    fi

    rm -f "${tmp}"
}

run_bench "ippan-mempool" "tx_validation" "Transaction validation throughput"
run_bench "ippan-consensus" "round_processing" "Consensus round execution"
run_bench "ippan-ai-core" "dgbdt_scoring" "D-GBDT scoring throughput"
run_bench "ippan-time" "hashtimer_ordering" "HashTimer ordering/sorting"

echo "Benchmark artifacts stored in ${REPORT_FILE}"
