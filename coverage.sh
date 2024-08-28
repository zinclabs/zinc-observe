#!/usr/bin/env bash
set -eu -o pipefail
# set -x
export PS4='+ [${BASH_SOURCE[0]##*/}:${LINENO}${FUNCNAME[0]:+:${FUNCNAME[0]}}] '

export COVERAGE_FUNCTIONS=${COVERAGE_FUNCTIONS:-30}
export COVERAGE_LINES=${COVERAGE_LINES:-30}
export COVERAGE_REGIONS=${COVERAGE_REGIONS:-19}

usage() {
    cat <<EOF
Usage: $0 [-h | --help] [<command>]

Commands:
  check [TESTNAME]  Run 'cargo llvm-cov test' and verify that code coverage statistics
                    are greater than or equal to the thresholds. Test name filtering
                    is supported.
  help              Show this help and exit.
  html              Generate coverage report in HTML format and open it in the default
                    browser.
  show-env          Show the coverage thresholds.

Omitted <command> defaults to 'check'.
EOF
}

_cov_test() {
    cargo llvm-cov --version >/dev/null || cargo install cargo-llvm-cov
    cargo llvm-cov test \
        --verbose \
        --ignore-filename-regex job \
        --ignore-run-fail \
        "$@"
}

cmd_html() {
    _cov_test --html "$@"
    open target/llvm-cov/html/index.html  # HACK: `open` is not portable
}

cmd_show_env() {
    cat <<EOF
COVERAGE_FUNCTIONS=$COVERAGE_FUNCTIONS
COVERAGE_LINES=$COVERAGE_LINES
COVERAGE_REGIONS=$COVERAGE_REGIONS
EOF
}

cmd_check() {
    _cov_test --json --summary-only --output-path report.json "$@"

    python3 <(
    cat <<'EOF'
import json
import os
import sys

thresholds = {
    'functions': float(os.environ['COVERAGE_FUNCTIONS']),
    'lines': float(os.environ['COVERAGE_LINES']),
    'regions': float(os.environ['COVERAGE_REGIONS']),
}

with open('report.json') as f:
    report = json.load(f)

totals = report['data'][0]['totals']

exit_status = 0
for k, threshold in thresholds.items():
    actual = totals[k]['percent']
    k = k.capitalize()
    if actual >= threshold:
        print(f'✅ {k} coverage: {actual:.2f}%', file=sys.stderr)
    else:
        print(
            f'❌ {k} coverage is below threshold: {actual:.2f}% < {threshold}%',
            file=sys.stderr,
        )
        exit_status = 1
sys.exit(exit_status)
EOF
)
}

main() {
    case "${1:-}" in
        '')
            cmd_check
            ;;
        check)
            shift
            cmd_check "$@"
            ;;
        html)
            shift
            cmd_html "$@"
            ;;
        show-env)
            cmd_show_env
            ;;
        -h|--help|help)
            usage
            ;;
        *)
            echo >&2 "Invalid argument: $1"
            echo >&2 "Type '$0 --help' for usage"
            exit 1
            ;;
    esac
}

main "$@"
