# AILOOP Test Results Report
Generated: 2026-01-28T15:16:43Z
Command: ./scripts/run-tests.sh
Output File: ./.newton/state/evaluator_status.md
JSON File: ./.newton/state/test_output.json

## Overall Status
✅ **PASSED** - All tests completed successfully

## Test Statistics
- **Total Tests:** 200
- **Passed:** 200
- **Failed:** 0
- **Skipped:** 0
- **Passing Rate:** 100%

## Progress Visualization
```
[██████████████████████████████] 100% (200/200)
```

## Performance
- **Test Duration:** 10.050s

## Files
- **Raw Test Output:** `./.newton/state/test_output.json`
- **Markdown Report:** `./.newton/state/evaluator_status.md`

## Raw Test Output
Complete test output is saved in: `./.newton/state/test_output.json`

You can analyze it with standard Unix tools:
```bash
# Count total tests
grep -c 'PASS\|FAIL\|SKIP' ./.newton/state/test_output.json

# Show failed tests
grep -A 2 -B 2 'FAIL' ./.newton/state/test_output.json
```
