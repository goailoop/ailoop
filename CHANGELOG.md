# Changelog

## [1.0.0] - 2026-05-08

### Breaking Changes

- **CLI subcommand removed:** `ailoop workflow` and all sub-subcommands (`list`, `run`, `status`, `cancel`) have been removed. Invocations will fail with exit code 2 and an "unrecognized subcommand 'workflow'" message. Use external orchestrators (Newton, CI systems) instead.

- **Wire-protocol variants removed:** Messages with `content.type` of `"workflow_progress"`, `"workflow_completed"`, `"stdout"`, or `"stderr"` will fail deserialization on the upgraded server (HTTP 400). Clients sending these types must be updated.

- **Rust library API break:** The `ailoop_core::workflow` module and all types in `ailoop_core::models::workflow` (`WorkflowDefinition`, `WorkflowState`, `ExecutionStatus`, `StateTransition`, `WorkflowExecutionInstance`, `ApprovalRequest`, `TimeoutBehavior`, `OutputType`, `ExecutionOutput`, etc.) have been deleted. Downstream Rust crates depending on `ailoop-core` must remove all import sites.

- **Workspace dependency removal:** `serde_yaml`, `fs2`, `crossbeam-queue`, and `jsonschema` have been removed from `[workspace.dependencies]`. Downstream crates that inherited these implicitly must add them as direct dependencies if still needed.

- **UI duplicate files removed:** `ailoop-core/assets/ailoop-ui.html` and `web/ailoop-ui.html` (unused duplicates, not compiled into the binary) have been deleted. The embedded UI remains at `ailoop-server/assets/ailoop-ui.html`.

### Migration Guide

#### Replacing `ailoop workflow`

The `ailoop workflow` CLI subcommand has been removed. Replace workflow orchestration with external tools:

- **Newton** — for multi-step AI agent orchestration
- **GitHub Actions / CI systems** — for automated pipeline execution
- **Shell scripts** — for simple sequential bash workflows

#### Removing stale persistence files

`~/.ailoop/workflow_store.json` and `~/.ailoop/workflows/` may exist on your system from prior versions. These files are no longer written to and can be safely removed manually:

```sh
rm -f ~/.ailoop/workflow_store.json
rm -rf ~/.ailoop/workflows/
```

This is optional — their presence does not affect ailoop operation.

#### SDK consumers

ailoop-py and ailoop-js SDKs: the `MessageContent` type no longer includes `workflow_progress`, `workflow_completed`, `stdout`, or `stderr` variants. Update SDK consumers to remove handling for these types.

### Retained Functionality

All HIL messaging paths are unchanged: `ask`, `authorize`, `say`, `navigate`, `notification`, and all task operations (`task_create`, `task_update`, `task_dependency_add`, `task_dependency_remove`) continue to work without modification.
