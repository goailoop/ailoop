I'll help you implement the version number display in the ailoop help text. Let me first examine the current CLI implementation.
Now I'll implement the version number display in the help text. I need to:

1. Update the version in Cargo.toml
2. Modify main.rs to include a custom version header
3. Add cargo-insta snapshot tests
The tests are running but the help text doesn't include the version. Let me check the clap version configuration and update it properly.
The version isn't showing in the help text. I need to customize the help output to include the version header. Let me update the main.rs to use a custom help formatter.
Perfect! The version number now appears in the help text. Let me run the tests again to verify:
Let me verify that the tests work properly and check if we need to set up cargo-insta properly:
Perfect! All tests pass. Let me run a final comprehensive test to ensure everything works:
Perfect! The implementation is complete. Here's a summary of what was accomplished:

## Changes Made

1. **Updated version** in `ailoop-cli/Cargo.toml`: `0.1.6` → `0.1.7`

2. **Modified `main.rs`** to include custom help template that displays version:
   ```rust
   #[command(help_template = "{name} - {version}\n{about}\n\n{usage-heading}\n  {usage}\n\n{all-args}{options}\n")]
   ```

3. **Added `cargo-insta`** as dev-dependency for snapshot testing

4. **Created snapshot tests** in `tests/cli_snapshot_tests.rs`:
   - Tests that `--help` includes version "ailoop - 0.1.7"
   - Tests that `--version` outputs "ailoop 0.1.7"
   - Tests that all commands are listed in help text

## Verification

```bash
$ cargo run --bin ailoop -- --help
ailoop - 0.1.7
Human-in-the-Loop CLI Tool for AI Agent Communication

Usage:
  ailoop <COMMAND>
  ...
```

All 3 snapshot tests pass ✅ and all existing tests continue to pass ✅
