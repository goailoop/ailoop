#!/bin/bash

# AILOOP Test Runner Script
# Matches CI: Rust (fmt, clippy, workspace tests, all-targets), Python (mypy, ruff, pytest, build),
# TypeScript (type-check, lint, test with coverage, build). Run before push to catch the same
# failures as CI. Python/TS output is streamed to stderr and captured for report.
#
# Note: Do not add Rust tests that run bash commands without skipping on Windows. Bash is
# unavailable or unreliable on Windows CI; use #[cfg(not(target_os = "windows"))] on any test
# that runs bash (e.g. BashExecutor, echo, exit, sleep in commands).
#
# Usage: ./run-tests.sh [OPTIONS]
#
# Options:
#   -f, --format FORMAT  Output format: text (default) or json. Report goes to stdout.
#   -o, --output FILE    Optional: write markdown report to FILE.
#   -j, --json FILE      Optional: write JSON results to FILE.
#   -h, --help           Show this help message.
#
# Default: text report to stdout only. Use -o/-j to write to files.

set -e

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
OUTPUT_FORMAT="text"
OUTPUT_FILE=""
JSON_FILE=""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print error and exit
error_exit() {
    echo -e "${RED}Error: $1${NC}" >&2
    echo "Usage: $0 [OPTIONS]" >&2
    echo "" >&2
    echo "Options:" >&2
    echo "  -f, --format FORMAT   Output format: text (default) or json. Report to stdout." >&2
    echo "  -o, --output FILE     Optional: write markdown report to FILE." >&2
    echo "  -j, --json FILE       Optional: write JSON results to FILE." >&2
    echo "  -h, --help            Show this help message." >&2
    exit 1
}

# Function to check if command exists
check_command() {
    local cmd=$1
    local description=$2
    if ! command -v "$cmd" >/dev/null 2>&1; then
        error_exit "$description ($cmd) is not installed or not in PATH. Please install it first."
    fi
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -f|--format)
            if [[ "$2" != "text" && "$2" != "json" ]]; then
                error_exit "Format must be 'text' or 'json', got: $2"
            fi
            OUTPUT_FORMAT="$2"
            shift 2
            ;;
        -o|--output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        -j|--json)
            JSON_FILE="$2"
            shift 2
            ;;
        -h|--help)
            echo "AILoop Test Runner Script"
            echo ""
            echo "Runs Rust/Python/TypeScript checks and tests; emits report to stdout (text or JSON)."
            echo ""
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -f, --format FORMAT   Output format: text (default) or json. Report goes to stdout."
            echo "  -o, --output FILE     Optional: write markdown report to FILE."
            echo "  -j, --json FILE       Optional: write JSON results to FILE."
            echo "  -h, --help            Show this help message."
            echo ""
            echo "Default: text report to stdout only. -o and -j are optional and only write when a path is given."
            echo ""
            echo "Requirements:"
            echo "  - Rust (rustfmt, clippy): same as CI"
            echo "  - uv: for ailoop-py (uv sync, uv run mypy/ruff/pytest)"
            echo "  - Node.js + npm: for ailoop-js (cd ailoop-js && npm ci)"
            exit 0
            ;;
        *)
            error_exit "Unknown option: $1"
            ;;
    esac
done

# Check dependencies
echo -e "${YELLOW}Checking dependencies...${NC}" >&2

check_command "cargo" "Cargo (Rust package manager)"

echo -e "${GREEN}All dependencies found!${NC}" >&2
echo "" >&2

# Change to the AILoop directory (assuming script is run from there)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AILOOP_DIR="$(dirname "$SCRIPT_DIR")"

echo -e "${YELLOW}Running tests in: $AILOOP_DIR${NC}" >&2
cd "$AILOOP_DIR"

# Optional: version check (CI runs scripts/check-versions.sh)
VERSION_EXIT=0
if [ -f "scripts/check-versions.sh" ]; then
    echo -e "${YELLOW}Checking package versions...${NC}" >&2
    if bash scripts/check-versions.sh >/dev/null 2>&1; then
        echo -e "${GREEN}Versions OK${NC}" >&2
    else
        VERSION_EXIT=1
        echo -e "${RED}Version check failed (run scripts/check-versions.sh for details)${NC}" >&2
    fi
fi

# Rust: match CI (fmt, clippy, workspace tests, all-targets)
set +e
echo -e "${YELLOW}Running cargo fmt --check...${NC}" >&2
FMT_OUTPUT=$(cargo fmt --check 2>&1)
FMT_EXIT=$?

echo -e "${YELLOW}Running cargo clippy --all-targets --all-features...${NC}" >&2
CLIPPY_OUTPUT=$(cargo clippy --all-targets --all-features -- -D warnings 2>&1)
CLIPPY_EXIT=$?

# Optional: on Linux, run clippy for Windows target to catch Windows-only errors (e.g. unused imports in cfg-gated tests)
if [ "$CLIPPY_EXIT" -eq 0 ] && [ "$(uname -s)" = "Linux" ]; then
    if rustup target list --installed 2>/dev/null | grep -q "x86_64-pc-windows-msvc"; then
        echo -e "${YELLOW}Running cargo clippy for Windows target (x86_64-pc-windows-msvc)...${NC}" >&2
        WCLIPPY=$(cargo clippy --target x86_64-pc-windows-msvc --all-targets --all-features -- -D warnings 2>&1)
        WEXIT=$?
        if [ "$WEXIT" -ne 0 ]; then
            CLIPPY_EXIT=1
            CLIPPY_OUTPUT="$CLIPPY_OUTPUT

--- clippy for Windows target (x86_64-pc-windows-msvc) ---
$WCLIPPY"
        fi
    fi
fi

# Pre-flight check: verify integration test readiness
echo -e "${YELLOW}Running pre-flight checks for integration tests...${NC}" >&2
PREFLIGHT_EXIT=0

# Check for port availability issues (integration tests need to bind ports)
if command -v netstat >/dev/null 2>&1; then
    LISTENING_PORTS=$(netstat -an 2>/dev/null | grep -c LISTEN || echo "0")
    if [ "$LISTENING_PORTS" -gt 5000 ]; then
        echo -e "${YELLOW}Warning: High number of listening ports detected ($LISTENING_PORTS).${NC}" >&2
        echo -e "${YELLOW}This may cause integration test failures due to port exhaustion.${NC}" >&2
    fi
elif command -v ss >/dev/null 2>&1; then
    LISTENING_PORTS=$(ss -tln 2>/dev/null | grep -c LISTEN || echo "0")
    if [ "$LISTENING_PORTS" -gt 5000 ]; then
        echo -e "${YELLOW}Warning: High number of listening ports detected ($LISTENING_PORTS).${NC}" >&2
        echo -e "${YELLOW}This may cause integration test failures due to port exhaustion.${NC}" >&2
    fi
fi

# Check for processes that might interfere with tests
if pgrep -f "ailoop.*server" >/dev/null 2>&1; then
    echo -e "${YELLOW}Warning: Found running ailoop server processes that may interfere with tests.${NC}" >&2
    echo -e "${YELLOW}Consider stopping them before running tests.${NC}" >&2
fi

echo -e "${GREEN}Pre-flight checks completed${NC}" >&2

echo -e "${YELLOW}Running cargo test --workspace --verbose...${NC}" >&2
TEST_OUTPUT=$(cargo test --workspace --verbose 2>&1)
RUST_TEST_EXIT=$?

echo -e "${YELLOW}Running cargo test --workspace --all-targets...${NC}" >&2
ALL_TARGETS_OUTPUT=$(cargo test --workspace --all-targets 2>&1)
ALL_TARGETS_EXIT=$?
set -e

RUST_EXIT=0
[ "$FMT_EXIT" -ne 0 ] && RUST_EXIT=1 || true
[ "$CLIPPY_EXIT" -ne 0 ] && RUST_EXIT=1 || true
[ "$RUST_TEST_EXIT" -ne 0 ] && RUST_EXIT=1 || true
[ "$ALL_TARGETS_EXIT" -ne 0 ] && RUST_EXIT=1 || true

if [ "$RUST_EXIT" -eq 0 ]; then
    echo -e "${GREEN}Rust checks and tests passed${NC}" >&2
else
    echo -e "${RED}Rust failed (fmt/clippy/tests/all-targets)${NC}" >&2
fi

# Python: match CI (mypy, ruff, pytest, build); stream output and capture for report
PYTHON_EXIT=0
PYTHON_OUTPUT=""
if [ -d "ailoop-py" ] && [ -f "ailoop-py/pyproject.toml" ]; then
    if ! command -v uv >/dev/null 2>&1; then
        error_exit "uv is required for ailoop-py (install: curl -LsSf https://astral.sh/uv/install.sh | sh)"
    fi
    echo "" >&2
    echo -e "${YELLOW}Running Python SDK checks (mypy, ruff, pytest, build)...${NC}" >&2
    PY_TMP=$(mktemp)
    (
        cd ailoop-py &&
        uv sync --extra dev &&
        uv run mypy src/ailoop/ --ignore-missing-imports &&
        uv run ruff check src/ailoop/ &&
        uv run python -m pytest tests/ -v --cov=ailoop --cov-report=xml &&
        uv pip install -q .
    ) 2>&1 | tee "$PY_TMP"
    PYTHON_EXIT=${PIPESTATUS[0]}
    PYTHON_OUTPUT=$(cat "$PY_TMP")
    rm -f "$PY_TMP"
    if [ "$PYTHON_EXIT" -eq 0 ]; then
        echo -e "${GREEN}Python SDK checks passed${NC}" >&2
    else
        echo -e "${RED}Python SDK checks failed${NC}" >&2
    fi
fi

# TypeScript: match CI (type-check, lint, test, build); stream output and capture for report
TS_EXIT=0
TS_OUTPUT=""
if [ -d "ailoop-js" ] && [ -f "ailoop-js/package.json" ]; then
    echo "" >&2
    echo -e "${YELLOW}Running TypeScript SDK checks (type-check, lint, test, build)...${NC}" >&2
    TS_TMP=$(mktemp)
    (cd ailoop-js && npm ci --no-audit --no-fund --quiet && npm run type-check && npm run lint && npm test -- --coverage --watchAll=false && npm run build) 2>&1 | tee "$TS_TMP"
    TS_EXIT=${PIPESTATUS[0]}
    TS_OUTPUT=$(cat "$TS_TMP")
    rm -f "$TS_TMP"
    if [ "$TS_EXIT" -eq 0 ]; then
        echo -e "${GREEN}TypeScript SDK checks passed${NC}" >&2
    else
        echo -e "${RED}TypeScript SDK checks failed${NC}" >&2
    fi
fi

# Overall status: fail if any suite failed
EXIT_CODE=0
if [ "$VERSION_EXIT" -ne 0 ] || [ "$RUST_EXIT" -ne 0 ] || [ "$PYTHON_EXIT" -ne 0 ] || [ "$TS_EXIT" -ne 0 ]; then
    OVERALL_STATUS="FAILED"
    EXIT_CODE=1
    echo "" >&2
    echo -e "${RED}Some test suites failed!${NC}" >&2
else
    OVERALL_STATUS="PASSED"
    echo "" >&2
    echo -e "${GREEN}All test suites passed!${NC}" >&2
fi

echo "" >&2

# Parse test results from output
echo -e "${YELLOW}Parsing test results...${NC}" >&2

# Parse Rust test results: cargo test outputs "test result: ok. N passed; M failed; K ignored"
if echo "$TEST_OUTPUT" | grep -q "error\[" || echo "$TEST_OUTPUT" | grep -q "could not compile"; then
    echo -e "${YELLOW}Compilation errors detected - no tests could run${NC}" >&2
    PASSED=0
    FAILED=0
    SKIPPED=0
    TOTAL=0
    PASSING_PERCENTAGE=0
    STATS_AVAILABLE=false
    COMPILATION_FAILED=true
else
    COMPILATION_FAILED=false
    RESULT_LINE=$(echo "$TEST_OUTPUT" | grep "test result:" | tail -1)
    if [ -n "$RESULT_LINE" ]; then
        PASSED=$(echo "$RESULT_LINE" | sed -n 's/.* \([0-9]*\) passed.*/\1/p'); PASSED=${PASSED:-0}
        FAILED=$(echo "$RESULT_LINE" | sed -n 's/.* \([0-9]*\) failed.*/\1/p'); FAILED=${FAILED:-0}
        SKIPPED=$(echo "$RESULT_LINE" | sed -n 's/.* \([0-9]*\) ignored.*/\1/p'); SKIPPED=${SKIPPED:-0}
        TOTAL=$((PASSED + FAILED + SKIPPED))
        PASSING_PERCENTAGE=$((TOTAL > 0 ? PASSED * 100 / TOTAL : 0))
        STATS_AVAILABLE=true
    else
        PASSED=0; FAILED=0; SKIPPED=0; TOTAL=0; PASSING_PERCENTAGE=0
        STATS_AVAILABLE=true
    fi
fi

# Get failed test names (if any) from cargo test output
FAILED_TESTS=""
PORT_CONFLICT_DETECTED=false
if [ "${FAILED:-0}" -gt 0 ]; then
    FAILED_TESTS=$(echo "$TEST_OUTPUT" | grep -E "FAILED|failed" | head -10)

    # Check for port conflicts in test output
    if echo "$TEST_OUTPUT" | grep -qi "address already in use\|Address in use\|bind.*failed"; then
        PORT_CONFLICT_DETECTED=true
        echo -e "${RED}Port conflict detected in test failures!${NC}" >&2
        echo -e "${YELLOW}Integration tests failed due to port binding issues.${NC}" >&2
        echo -e "${YELLOW}This can happen when:${NC}" >&2
        echo -e "${YELLOW}  1. Another process is using required ports${NC}" >&2
        echo -e "${YELLOW}  2. Previous test runs didn't clean up properly${NC}" >&2
        echo -e "${YELLOW}  3. System is under heavy load${NC}" >&2
        echo -e "${YELLOW}Suggested fixes:${NC}" >&2
        echo -e "${YELLOW}  - Kill any running ailoop server processes${NC}" >&2
        echo -e "${YELLOW}  - Wait a few seconds and retry${NC}" >&2
        echo -e "${YELLOW}  - Check for port exhaustion with: netstat -an | grep -c LISTEN${NC}" >&2
        echo "" >&2
    fi
fi

# Escape JSON string (one line, truncate long output)
json_escape() { echo "$1" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/\\t/g; s/\r//g' | tr '\n' ' ' | head -c 2000; }

PYTHON_JSON=""
TS_JSON=""
[ "$PYTHON_EXIT" -ne 0 ] && [ -n "$PYTHON_OUTPUT" ] && PYTHON_JSON="\"python_failure\": \"$(json_escape "$PYTHON_OUTPUT")\", "
[ "$TS_EXIT" -ne 0 ] && [ -n "$TS_OUTPUT" ] && TS_JSON="\"typescript_failure\": \"$(json_escape "$TS_OUTPUT")\", "

# Build JSON content (for stdout and/or file)
PORT_CONFLICT_JSON=""
[ "$PORT_CONFLICT_DETECTED" = true ] && PORT_CONFLICT_JSON="\"port_conflict_detected\": true, "

if [ "$COMPILATION_FAILED" = true ]; then
    JSON_CONTENT=$(cat << EOF
{
  "status": "compilation_failed",
  "timestamp": "$TIMESTAMP",
  "exit_code": $EXIT_CODE,
  ${PORT_CONFLICT_JSON}
  "checks": { "version": $([ "$VERSION_EXIT" -eq 0 ] && echo true || echo false), "rust_fmt": $([ "$FMT_EXIT" -eq 0 ] && echo true || echo false), "rust_clippy": $([ "$CLIPPY_EXIT" -eq 0 ] && echo true || echo false), "rust_tests": false, "python": $([ "$PYTHON_EXIT" -eq 0 ] && echo true || echo false), "typescript": $([ "$TS_EXIT" -eq 0 ] && echo true || echo false) },
  "test_statistics": { "total": 0, "passed": 0, "failed": 0, "skipped": 0, "passing_percentage": 0 }
}
EOF
)
else
    JSON_CONTENT=$(cat << EOF
{
  "status": "$([ "$EXIT_CODE" -eq 0 ] && echo "passed" || echo "failed")",
  "timestamp": "$TIMESTAMP",
  "exit_code": $EXIT_CODE,
  ${PORT_CONFLICT_JSON}
  "checks": { "version": $([ "$VERSION_EXIT" -eq 0 ] && echo true || echo false), "rust_fmt": $([ "$FMT_EXIT" -eq 0 ] && echo true || echo false), "rust_clippy": $([ "$CLIPPY_EXIT" -eq 0 ] && echo true || echo false), "rust_tests": $([ "$RUST_EXIT" -eq 0 ] && echo true || echo false), "python": $([ "$PYTHON_EXIT" -eq 0 ] && echo true || echo false), "typescript": $([ "$TS_EXIT" -eq 0 ] && echo true || echo false) },
  ${PYTHON_JSON}${TS_JSON}
  "test_statistics": { "total": $TOTAL, "passed": ${PASSED:-0}, "failed": ${FAILED:-0}, "skipped": ${SKIPPED:-0}, "passing_percentage": $PASSING_PERCENTAGE }
}
EOF
)
fi

# Progress message
if [ -n "$OUTPUT_FILE" ] || [ -n "$JSON_FILE" ]; then
    echo -e "${YELLOW}Generating report...${NC}" >&2
    [ -n "$OUTPUT_FILE" ] && echo -e "${YELLOW}  Markdown: $OUTPUT_FILE${NC}" >&2
    [ -n "$JSON_FILE" ] && echo -e "${YELLOW}  JSON: $JSON_FILE${NC}" >&2
else
    echo -e "${YELLOW}Generating report (stdout, format: $OUTPUT_FORMAT)...${NC}" >&2
fi

# Emit report: stdout and/or files
if [ "$OUTPUT_FORMAT" = "json" ]; then
    echo "$JSON_CONTENT"
    if [ -n "$JSON_FILE" ]; then
        mkdir -p "$(dirname "$JSON_FILE")"
        echo "$JSON_CONTENT" > "$JSON_FILE"
    fi
else
    print_text_report() {
        echo "# AILOOP Test Results Report"
        echo "Generated: $TIMESTAMP"
        echo "Command: $0"
        echo ""

        echo "## Overall Status"
        if [ "$COMPILATION_FAILED" = true ]; then
            echo "COMPILATION FAILED - Code does not compile, tests cannot run"
        elif [ "$EXIT_CODE" -eq 0 ]; then
            echo "PASSED - All tests completed successfully"
        else
            echo "FAILED - Some tests failed"
        fi
        echo ""

        echo "## Checks (match CI)"
        echo "- Version: $([ "$VERSION_EXIT" -eq 0 ] && echo 'PASSED' || echo 'FAILED')"
        echo "- Rust fmt: $([ "$FMT_EXIT" -eq 0 ] && echo 'PASSED' || echo 'FAILED')"
        echo "- Rust clippy: $([ "$CLIPPY_EXIT" -eq 0 ] && echo 'PASSED' || echo 'FAILED')"
        echo "- Rust tests: $([ "$RUST_EXIT" -eq 0 ] && echo 'PASSED' || echo 'FAILED')"
        if [ -d "ailoop-py" ] && [ -f "ailoop-py/pyproject.toml" ]; then
            echo "- Python: $([ "$PYTHON_EXIT" -eq 0 ] && echo 'PASSED' || echo 'FAILED')"
        fi
        if [ -d "ailoop-js" ] && [ -f "ailoop-js/package.json" ]; then
            echo "- TypeScript: $([ "$TS_EXIT" -eq 0 ] && echo 'PASSED' || echo 'FAILED')"
        fi
        echo ""

        if [ "$FMT_EXIT" -ne 0 ] && [ -n "$FMT_OUTPUT" ]; then
            echo "### Rust fmt failure"
            echo "$FMT_OUTPUT"
            echo ""
        fi
        if [ "$CLIPPY_EXIT" -ne 0 ] && [ -n "$CLIPPY_OUTPUT" ]; then
            echo "### Rust clippy failure"
            echo "$CLIPPY_OUTPUT"
            echo ""
        fi
        if [ "$PYTHON_EXIT" -ne 0 ] && [ -n "$PYTHON_OUTPUT" ]; then
            echo "### Python SDK failure"
            echo "$PYTHON_OUTPUT"
            echo ""
        fi
        if [ "$TS_EXIT" -ne 0 ] && [ -n "$TS_OUTPUT" ]; then
            echo "### TypeScript SDK failure"
            echo "$TS_OUTPUT"
            echo ""
        fi

        echo "## Test Statistics"
        if [ "$COMPILATION_FAILED" = true ]; then
            echo "- Status: Compilation failed - no tests executed"
            echo "- Total Tests: N/A"
            echo "- Passed: N/A"
            echo "- Failed: N/A"
            echo "- Skipped: N/A"
            echo "- Passing Rate: N/A"
        else
            echo "- Total Tests: $TOTAL"
            echo "- Passed: ${PASSED:-0}"
            echo "- Failed: ${FAILED:-0}"
            echo "- Skipped: ${SKIPPED:-0}"
            echo "- Passing Rate: ${PASSING_PERCENTAGE}%"
        fi
        echo ""

        if [ "$COMPILATION_FAILED" = true ]; then
            echo "## Progress Visualization"
            echo "[..............................] COMPILATION FAILED"
            echo ""
        elif [ "$TOTAL" -gt 0 ]; then
            echo "## Progress Visualization"
            BAR_WIDTH=30
            FILLED=$((PASSED * BAR_WIDTH / TOTAL))
            EMPTY=$((BAR_WIDTH - FILLED))
            printf "["
            for ((i=0; i<FILLED; i++)); do printf "#"; done
            for ((i=0; i<EMPTY; i++)); do printf "."; done
            printf "] %d%% (%d/%d)\n" "$PASSING_PERCENTAGE" "${PASSED:-0}" "$TOTAL"
            echo ""
        else
            echo "## Progress Visualization"
            echo "[..............................] No tests found"
            echo ""
        fi

        if [ -n "$FAILED_TESTS" ] && [ "${FAILED:-0}" -gt 0 ]; then
            echo "## Failed Tests"
            echo ""
            echo "$FAILED_TESTS"
            echo ""
            if [ "$PORT_CONFLICT_DETECTED" = true ]; then
                echo "### Port Conflict Detected"
                echo "Integration tests failed due to port binding issues (Address already in use)."
                echo "Suggested fixes: pkill -f 'ailoop.*server'; wait and retry; check port exhaustion."
                echo ""
            fi
        fi

        DURATION_LINE=$(echo "$TEST_OUTPUT" | grep "Summary.*\[" | head -1)
        if [ -n "$DURATION_LINE" ]; then
            DURATION=$(echo "$DURATION_LINE" | sed -n 's/.*\[\s*\([0-9.]*\)s\].*/\1/p')
            if [ -n "$DURATION" ]; then
                echo "## Performance"
                echo "- Test Duration: ${DURATION}s"
                echo ""
            fi
        fi
    }

    print_text_report
    if [ -n "$OUTPUT_FILE" ]; then
        mkdir -p "$(dirname "$OUTPUT_FILE")"
        {
            print_text_report
            echo "## Files"
            echo "- Markdown Report: $OUTPUT_FILE"
            [ -n "$JSON_FILE" ] && echo "- JSON results: $JSON_FILE"
            echo ""
        } > "$OUTPUT_FILE"
    fi
    if [ -n "$JSON_FILE" ]; then
        mkdir -p "$(dirname "$JSON_FILE")"
        echo "$JSON_CONTENT" > "$JSON_FILE"
    fi
fi

# Console summary (stderr)
if [ -n "$OUTPUT_FILE" ] || [ -n "$JSON_FILE" ]; then
    echo -e "${GREEN}Report generated successfully!${NC}" >&2
else
    echo -e "${GREEN}Report written to stdout.${NC}" >&2
fi
echo "" >&2

if [ "$COMPILATION_FAILED" = true ]; then
    echo "Summary: version=$([ "$VERSION_EXIT" -eq 0 ] && echo PASSED || echo FAILED), rust=FAILED, python=$([ "$PYTHON_EXIT" -eq 0 ] && echo PASSED || echo FAILED), typescript=$([ "$TS_EXIT" -eq 0 ] && echo PASSED || echo FAILED)" >&2
    if [ -n "$OUTPUT_FILE" ]; then
        echo -e "${RED}Code does not compile. Check $OUTPUT_FILE for details.${NC}" >&2
    else
        echo -e "${RED}Code does not compile. See report above.${NC}" >&2
    fi
else
    echo "Summary: version=$([ "$VERSION_EXIT" -eq 0 ] && echo PASSED || echo FAILED), rust=$([ "$RUST_EXIT" -eq 0 ] && echo PASSED || echo FAILED), python=$([ "$PYTHON_EXIT" -eq 0 ] && echo PASSED || echo FAILED), typescript=$([ "$TS_EXIT" -eq 0 ] && echo PASSED || echo FAILED)" >&2
    echo "Rust tests: $TOTAL total, ${PASSED:-0} passed, ${FAILED:-0} failed (${PASSING_PERCENTAGE}%)" >&2
    if [ "$EXIT_CODE" -eq 0 ]; then
        echo -e "${GREEN}All checks passed.${NC}" >&2
    else
        if [ -n "$OUTPUT_FILE" ]; then
            echo -e "${RED}Some checks failed. See $OUTPUT_FILE for full output.${NC}" >&2
        else
            echo -e "${RED}Some checks failed. See report above.${NC}" >&2
        fi
    fi
fi

echo "" >&2
if [ -n "$OUTPUT_FILE" ] || [ -n "$JSON_FILE" ]; then
    echo "Files created:" >&2
    [ -n "$OUTPUT_FILE" ] && echo "  Markdown report: $OUTPUT_FILE" >&2
    [ -n "$JSON_FILE" ] && echo "  JSON: $JSON_FILE" >&2
fi

exit $EXIT_CODE
