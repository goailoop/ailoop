#!/bin/bash

# AILOOP Test Runner Script
# Matches CI: Rust (fmt, clippy, workspace tests, all-targets), Python (mypy, ruff, pytest, build),
# TypeScript (type-check, lint, test with coverage, build). Run before push to catch the same
# failures as CI (e.g. ruff F841, jest coverage thresholds); full failure output is written
# to OUTPUT_FILE and stderr.
#
# Usage: ./run-tests.sh -o OUTPUT_FILE -j JSON_FILE [OPTIONS]
#
# Options:
#   -o, --output FILE    Output markdown report file (required)
#   -j, --json FILE      JSON results file (required)
#   -h, --help           Show this help message

set -e

# Required parameters
OUTPUT_FILE=""
JSON_FILE=""
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print error and exit
error_exit() {
    echo -e "${RED}Error: $1${NC}" >&2
    echo "Usage: $0 -o OUTPUT_FILE -j JSON_FILE [OPTIONS]" >&2
    echo "" >&2
    echo "Options:" >&2
    echo "  -o, --output FILE    Output markdown report file (required)" >&2
    echo "  -j, --json FILE      JSON results file (required)" >&2
    echo "  -h, --help           Show this help message" >&2
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
            echo "Runs tests with cargo-nextest, captures results, and generates statistics"
            echo ""
            echo "Usage: $0 -o OUTPUT_FILE -j JSON_FILE [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -o, --output FILE    Output markdown report file (required)"
            echo "  -j, --json FILE      JSON results file (required)"
            echo "  -h, --help           Show this help message"
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

# Validate required parameters
if [ -z "$OUTPUT_FILE" ]; then
    error_exit "Output file is required. Use -o or --output to specify the markdown report file."
fi

if [ -z "$JSON_FILE" ]; then
    error_exit "JSON file is required. Use -j or --json to specify the JSON results file."
fi

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

# Python: match CI (mypy, ruff, pytest, build); capture full output for failures
PYTHON_EXIT=0
PYTHON_OUTPUT=""
if [ -d "ailoop-py" ] && [ -f "ailoop-py/pyproject.toml" ]; then
    if ! command -v uv >/dev/null 2>&1; then
        error_exit "uv is required for ailoop-py (install: curl -LsSf https://astral.sh/uv/install.sh | sh)"
    fi
    echo "" >&2
    echo -e "${YELLOW}Running Python SDK checks (mypy, ruff, pytest, build)...${NC}" >&2
    PYTHON_OUTPUT=$( (
        cd ailoop-py &&
        uv sync &&
        uv run mypy src/ailoop/ --ignore-missing-imports &&
        uv run ruff check src/ailoop/ &&
        uv run pytest tests/ -v --cov=ailoop --cov-report=xml &&
        uv pip install -q .
    ) 2>&1 )
    PYTHON_EXIT=$?
    if [ "$PYTHON_EXIT" -eq 0 ]; then
        echo -e "${GREEN}Python SDK checks passed${NC}" >&2
    else
        echo -e "${RED}Python SDK checks failed${NC}" >&2
        echo "$PYTHON_OUTPUT" | tail -80 >&2
    fi
fi

# TypeScript: match CI (type-check, lint, test, build); capture full output for failures
TS_EXIT=0
TS_OUTPUT=""
if [ -d "ailoop-js" ] && [ -f "ailoop-js/package.json" ]; then
    echo "" >&2
    echo -e "${YELLOW}Running TypeScript SDK checks (type-check, lint, test, build)...${NC}" >&2
    TS_OUTPUT=$( (cd ailoop-js && npm ci --no-audit --no-fund --quiet && npm run type-check && npm run lint && npm test -- --coverage --watchAll=false && npm run build) 2>&1 )
    TS_EXIT=$?
    if [ "$TS_EXIT" -eq 0 ]; then
        echo -e "${GREEN}TypeScript SDK checks passed${NC}" >&2
    else
        echo -e "${RED}TypeScript SDK checks failed${NC}" >&2
        echo "$TS_OUTPUT" | tail -80 >&2
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
if [ "${FAILED:-0}" -gt 0 ]; then
    FAILED_TESTS=$(echo "$TEST_OUTPUT" | grep -E "FAILED|failed" | head -10)
fi

# Escape JSON string (one line, truncate long output)
json_escape() { echo "$1" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/\\t/g; s/\r//g' | tr '\n' ' ' | head -c 2000; }

PYTHON_JSON=""
TS_JSON=""
[ "$PYTHON_EXIT" -ne 0 ] && [ -n "$PYTHON_OUTPUT" ] && PYTHON_JSON="\"python_failure\": \"$(json_escape "$PYTHON_OUTPUT")\", "
[ "$TS_EXIT" -ne 0 ] && [ -n "$TS_OUTPUT" ] && TS_JSON="\"typescript_failure\": \"$(json_escape "$TS_OUTPUT")\", "

# Create structured JSON output
echo -e "${YELLOW}Generating JSON output...${NC}" >&2
if [ "$COMPILATION_FAILED" = true ]; then
    cat > "$JSON_FILE" << EOF
{
  "status": "compilation_failed",
  "timestamp": "$TIMESTAMP",
  "exit_code": $EXIT_CODE,
  "checks": { "version": $([ "$VERSION_EXIT" -eq 0 ] && echo true || echo false), "rust_fmt": $([ "$FMT_EXIT" -eq 0 ] && echo true || echo false), "rust_clippy": $([ "$CLIPPY_EXIT" -eq 0 ] && echo true || echo false), "rust_tests": false, "python": $([ "$PYTHON_EXIT" -eq 0 ] && echo true || echo false), "typescript": $([ "$TS_EXIT" -eq 0 ] && echo true || echo false) },
  "test_statistics": { "total": 0, "passed": 0, "failed": 0, "skipped": 0, "passing_percentage": 0 }
}
EOF
else
    cat > "$JSON_FILE" << EOF
{
  "status": "$([ "$EXIT_CODE" -eq 0 ] && echo "passed" || echo "failed")",
  "timestamp": "$TIMESTAMP",
  "exit_code": $EXIT_CODE,
  "checks": { "version": $([ "$VERSION_EXIT" -eq 0 ] && echo true || echo false), "rust_fmt": $([ "$FMT_EXIT" -eq 0 ] && echo true || echo false), "rust_clippy": $([ "$CLIPPY_EXIT" -eq 0 ] && echo true || echo false), "rust_tests": $([ "$RUST_EXIT" -eq 0 ] && echo true || echo false), "python": $([ "$PYTHON_EXIT" -eq 0 ] && echo true || echo false), "typescript": $([ "$TS_EXIT" -eq 0 ] && echo true || echo false) },
  ${PYTHON_JSON}${TS_JSON}
  "test_statistics": { "total": $TOTAL, "passed": ${PASSED:-0}, "failed": ${FAILED:-0}, "skipped": ${SKIPPED:-0}, "passing_percentage": $PASSING_PERCENTAGE }
}
EOF
fi

# Generate comprehensive report
echo -e "${YELLOW}Generating report: $OUTPUT_FILE${NC}" >&2

{
    echo "# AILOOP Test Results Report"
    echo "Generated: $TIMESTAMP"
    echo "Command: $0"
    echo "Output File: $OUTPUT_FILE"
    echo "JSON File: $JSON_FILE"
    echo ""

    echo "## Overall Status"
    if [ "$COMPILATION_FAILED" = true ]; then
        echo "❌ **COMPILATION FAILED** - Code does not compile, tests cannot run"
    elif [ "$EXIT_CODE" -eq 0 ]; then
        echo "✅ **PASSED** - All tests completed successfully"
    else
        echo "❌ **FAILED** - Some tests failed"
    fi
    echo ""

    echo "## Checks (match CI)"
    echo "- **Version (check-versions.sh):** $([ "$VERSION_EXIT" -eq 0 ] && echo 'PASSED' || echo 'FAILED')"
    echo "- **Rust fmt:** $([ "$FMT_EXIT" -eq 0 ] && echo 'PASSED' || echo 'FAILED')"
    echo "- **Rust clippy:** $([ "$CLIPPY_EXIT" -eq 0 ] && echo 'PASSED' || echo 'FAILED')"
    echo "- **Rust tests (workspace + all-targets):** $([ "$RUST_EXIT" -eq 0 ] && echo 'PASSED' || echo 'FAILED')"
    if [ -d "ailoop-py" ] && [ -f "ailoop-py/pyproject.toml" ]; then
        echo "- **Python (mypy, ruff, pytest, build):** $([ "$PYTHON_EXIT" -eq 0 ] && echo 'PASSED' || echo 'FAILED')"
    fi
    if [ -d "ailoop-js" ] && [ -f "ailoop-js/package.json" ]; then
        echo "- **TypeScript (type-check, lint, test, build):** $([ "$TS_EXIT" -eq 0 ] && echo 'PASSED' || echo 'FAILED')"
    fi
    echo ""

    if [ "$FMT_EXIT" -ne 0 ] && [ -n "$FMT_OUTPUT" ]; then
        echo "### Rust fmt failure"
        echo "\`\`\`"
        echo "$FMT_OUTPUT"
        echo "\`\`\`"
        echo ""
    fi
    if [ "$CLIPPY_EXIT" -ne 0 ] && [ -n "$CLIPPY_OUTPUT" ]; then
        echo "### Rust clippy failure"
        echo "\`\`\`"
        echo "$CLIPPY_OUTPUT"
        echo "\`\`\`"
        echo ""
    fi
    if [ "$PYTHON_EXIT" -ne 0 ] && [ -n "$PYTHON_OUTPUT" ]; then
        echo "### Python SDK failure (mypy / ruff / pytest / build)"
        echo "\`\`\`"
        echo "$PYTHON_OUTPUT"
        echo "\`\`\`"
        echo ""
    fi
    if [ "$TS_EXIT" -ne 0 ] && [ -n "$TS_OUTPUT" ]; then
        echo "### TypeScript SDK failure (type-check / lint / test / build)"
        echo "\`\`\`"
        echo "$TS_OUTPUT"
        echo "\`\`\`"
        echo ""
    fi

    echo "## Test Statistics"
    if [ "$COMPILATION_FAILED" = true ]; then
        echo "- **Status:** Compilation failed - no tests executed"
        echo "- **Total Tests:** N/A"
        echo "- **Passed:** N/A"
        echo "- **Failed:** N/A"
        echo "- **Skipped:** N/A"
        echo "- **Passing Rate:** N/A"
    else
        echo "- **Total Tests:** $TOTAL"
        echo "- **Passed:** $PASSED"
        echo "- **Failed:** $FAILED"
        echo "- **Skipped:** $SKIPPED"
        echo "- **Passing Rate:** ${PASSING_PERCENTAGE}%"
    fi
    echo ""

    # Progress bar visualization
    if [ "$COMPILATION_FAILED" = true ]; then
        echo "## Progress Visualization"
        echo "\`\`\`"
        echo "[░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] COMPILATION FAILED"
        echo "\`\`\`"
        echo ""
    elif [ "$TOTAL" -gt 0 ]; then
        echo "## Progress Visualization"
        BAR_WIDTH=30
        FILLED=$((PASSED * BAR_WIDTH / TOTAL))
        EMPTY=$((BAR_WIDTH - FILLED))

        echo "\`\`\`"
        printf "["
        for ((i=0; i<FILLED; i++)); do printf "█"; done
        for ((i=0; i<EMPTY; i++)); do printf "░"; done
        printf "] %d%% (%d/%d)\n" "$PASSING_PERCENTAGE" "$PASSED" "$TOTAL"
        echo "\`\`\`"
        echo ""
    else
        echo "## Progress Visualization"
        echo "\`\`\`"
        echo "[░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] No tests found"
        echo "\`\`\`"
        echo ""
    fi

    # Failed tests section
    if [ -n "$FAILED_TESTS" ] && [ "$FAILED" -gt 0 ]; then
        echo "## Failed Tests"
        echo ""
        echo "The following tests failed:"
        echo ""
        echo "\`\`\`"
        echo "$FAILED_TESTS"
        echo "\`\`\`"
        echo ""
    fi

    # Test duration (if available in summary)
    DURATION_LINE=$(echo "$TEST_OUTPUT" | grep "Summary.*\[" | head -1)
    if [ -n "$DURATION_LINE" ]; then
        DURATION=$(echo "$DURATION_LINE" | sed -n 's/.*\[\s*\([0-9.]*\)s\].*/\1/p')
        if [ -n "$DURATION" ]; then
            echo "## Performance"
            echo "- **Test Duration:** ${DURATION}s"
            echo ""
        fi
    fi

    echo "## Files"
    echo "- **Raw Test Output:** \`$JSON_FILE\`"
    echo "- **Markdown Report:** \`$OUTPUT_FILE\`"
    echo ""

    echo "## Raw Test Output"
    echo "Complete test output is saved in: \`$JSON_FILE\`"
    echo ""
    echo "You can analyze it with standard Unix tools:"
    echo "\`\`\`bash"
    echo "# Count total tests"
    echo "grep -c 'PASS\\|FAIL\\|SKIP' $JSON_FILE"
    echo ""
    echo "# Show failed tests"
    echo "grep -A 2 -B 2 'FAIL' $JSON_FILE"
    echo "\`\`\`"

} > "$OUTPUT_FILE"

# Console output summary
echo -e "${GREEN}Report generated successfully!${NC}" >&2
echo "" >&2

if [ "$COMPILATION_FAILED" = true ]; then
    echo "Summary: version=$([ "$VERSION_EXIT" -eq 0 ] && echo PASSED || echo FAILED), rust=FAILED, python=$([ "$PYTHON_EXIT" -eq 0 ] && echo PASSED || echo FAILED), typescript=$([ "$TS_EXIT" -eq 0 ] && echo PASSED || echo FAILED)" >&2
    echo -e "${RED}Code does not compile. Check $OUTPUT_FILE for details.${NC}" >&2
else
    echo "Summary: version=$([ "$VERSION_EXIT" -eq 0 ] && echo PASSED || echo FAILED), rust=$([ "$RUST_EXIT" -eq 0 ] && echo PASSED || echo FAILED), python=$([ "$PYTHON_EXIT" -eq 0 ] && echo PASSED || echo FAILED), typescript=$([ "$TS_EXIT" -eq 0 ] && echo PASSED || echo FAILED)" >&2
    echo "Rust tests: $TOTAL total, $PASSED passed, ${FAILED:-0} failed (${PASSING_PERCENTAGE}%)" >&2
    if [ "$EXIT_CODE" -eq 0 ]; then
        echo -e "${GREEN}All checks passed.${NC}" >&2
    else
        echo -e "${RED}Some checks failed. See $OUTPUT_FILE for full output (Python/TypeScript failures captured).${NC}" >&2
    fi
fi

echo "" >&2
echo "📁 Files created:" >&2
echo "  Markdown report: $OUTPUT_FILE" >&2
echo "  Raw output: $JSON_FILE" >&2

exit $EXIT_CODE
