# Changelog

## [Unreleased]

### Breaking Changes

- **Wire-protocol variant removed:** `content.type = "question"` is removed. Any consumer that deserialises messages of type `"question"` will receive a parse error on the upgraded server. Replace all usages with the new `"decision"` type.

- **Rust API break:** `MessageContent::Question` variant and `client::ask()` function have been removed from `ailoop-core`. Callers must migrate to `MessageContent::Decision` and `client::ask_decision()`.

- **TypeScript SDK break:** `QuestionContent` interface and `MessageFactory.createQuestion()` have been removed from `ailoop-js`. Use `DecisionContent` and `MessageFactory.createDecision()` instead.

- **Python SDK break:** `QuestionContent` class and `Message.create_question()` have been removed from `ailoop-py`. Use `DecisionContent`, `DecisionOption`, `DecisionRecommendation`, and `Message.create_decision()` instead.

- **CLI break:** The pipe-encoded question format (`"question|choice1|choice2"`) has been removed from `ailoop ask`. Use `--decision-json '<json>'` with a structured JSON body instead.

- **Response `answer` field:** Always contains the canonical `options[].id` string (not a raw label or index number). Consumers reading `answer` expecting a label must now read `metadata.label`; consumers expecting a number must read `metadata.index`.

- **Response `metadata` keys:** `"value"` renamed to `"label"`; new key `"option_id"` added (same as `answer`).

### Migration Guide

#### Python — `create_question()` → `create_decision()`

```python
# Before
from ailoop.models import Message
msg = Message.create_question(channel="ops", text="Which strategy?", choices=["blue-green", "canary"])

# After
from ailoop.models import Message, DecisionOption, DecisionRecommendation
msg = Message.create_decision(
    channel="ops",
    decision_id="deploy-strategy",
    summary="Which deployment strategy?",
    options=[
        DecisionOption(id="blue-green", label="Blue/Green"),
        DecisionOption(id="canary", label="Canary (10%)"),
    ],
    recommendation=DecisionRecommendation(option_id="blue-green"),
)
```

#### TypeScript — `createQuestion()` → `createDecision()`

```typescript
// Before
const msg = MessageFactory.createQuestion('ops', 'Which strategy?', 60, ['blue-green', 'canary']);

// After
const msg = MessageFactory.createDecision(
  'ops',
  'deploy-strategy',
  'Which deployment strategy?',
  [
    { id: 'blue-green', label: 'Blue/Green' },
    { id: 'canary', label: 'Canary (10%)' },
  ],
  300,
  undefined,
  { option_id: 'blue-green' }
);
```

#### Rust — `ask()` → `ask_decision()`

```rust
// Before
ailoop_core::client::ask(server_url, channel, "Which strategy?", Some(vec!["blue-green".into(), "canary".into()]), 60).await?;

// After
use ailoop_core::models::{DecisionOption, DecisionRecommendation};
ailoop_core::client::ask_decision(
    server_url,
    channel,
    "deploy-strategy".into(),
    "Which deployment strategy?".into(),
    None,
    vec![
        DecisionOption { id: "blue-green".into(), label: "Blue/Green".into(), detail_markdown: None },
        DecisionOption { id: "canary".into(), label: "Canary (10%)".into(), detail_markdown: None },
    ],
    Some(DecisionRecommendation { option_id: "blue-green".into(), rationale_markdown: None }),
    300,
).await?;
```

#### CLI — pipe syntax → `--decision-json`

```bash
# Before
ailoop ask "Which strategy?|blue-green|canary"

# After
ailoop ask --decision-json '{
  "decision_id": "deploy-strategy",
  "summary": "Which deployment strategy?",
  "options": [
    {"id": "blue-green", "label": "Blue/Green"},
    {"id": "canary", "label": "Canary (10%)"}
  ]
}'
```

---

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
