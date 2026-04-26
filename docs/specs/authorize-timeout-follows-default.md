# Authorize Timeout Follows Default

## Current vs Target Behavior

### Direct Mode

| Scenario | Current | Target |
|----------|---------|--------|
| `--default yes` + timeout | DENIED (exit 1) | GRANTED (exit 0) |
| `--default no` + timeout | DENIED (exit 1) | DENIED (exit 1) |
| `--default yes` + explicit `yes` | GRANTED (exit 0) | GRANTED (exit 0) |
| `--default yes` + explicit `no` | DENIED (exit 1) | DENIED (exit 1) |
| `--default yes` + Enter (empty) | GRANTED (exit 0) | GRANTED (exit 0) |
| `--default no` + Enter (empty) | DENIED (exit 1) | DENIED (exit 1) |
| Ctrl+C / Cancelled | DENIED (exit 1) | DENIED (exit 1) |

### Server Mode

| Scenario | Current | Target |
|----------|---------|--------|
| Server returns `Timeout` + `--default yes` | DENIED (exit 1) | GRANTED (exit 0) |
| Server returns `Timeout` + `--default no` | DENIED (exit 1) | DENIED (exit 1) |
| Server returns `Cancelled` | DENIED (exit 1) | DENIED (exit 1) |
| No response (None) + `--default yes` | DENIED (exit 1) | GRANTED (exit 0) |
| No response (None) + `--default no` | DENIED (exit 1) | DENIED (exit 1) |

## Non-Goals

- No change to explicit `yes`/`no` input handling.
- Cancel / Ctrl+C behavior remains denied/cancelled regardless of `--default`.
- No change to terminal countdown mechanics in `terminal_input.rs`.
- No change to server-side timeout logic.

## Implementation

A single `timeout_decision(default_yes: bool) -> AuthorizationDecision` helper centralizes the
policy: `Approved` when `default_yes=true`, `Denied` otherwise. All `InputResult::Timeout` and
server `ResponseType::Timeout` branches call this helper instead of hardcoding `Denied`.

## Acceptance Criteria

- [ ] `ailoop authorize <action> --default yes --timeout N` exits 0 when N seconds elapse with no input
- [ ] `ailoop authorize <action> --default no --timeout N` exits non-zero when N seconds elapse with no input
- [ ] Explicit `yes`/`no` input still overrides the default regardless of timeout setting
- [ ] Cancel (Ctrl+C) still exits non-zero regardless of `--default` setting
- [ ] Server-mode: `ResponseType::Timeout` follows `--default` flag
- [ ] No fmt/clippy regressions

## Regression Matrix

| Test | Expected Exit |
|------|--------------|
| `--default yes` + timeout | 0 |
| `--default no` + timeout | 1 |
| `--default yes` + empty input | 0 |
| `--default no` + empty input | 1 |
| explicit `yes` with `--default no` | 0 |
| explicit `no` with `--default yes` | 1 |
| Cancelled | 1 |
