# REQ — ailoop-ui decision surface (embedded server UI)

**Tracking:** [goailoop/ailoop#47](https://github.com/goailoop/ailoop/issues/47)

## Status

Draft requirement for `goailoop/ailoop`. Complements structured `MessageContent::Decision` on the wire.

## Problem

[`ailoop-server/assets/ailoop-ui.html`](../ailoop-server/assets/ailoop-ui.html) must present `decision_id`, `summary`, `context_markdown`, many `options` (each with optional `detail_markdown`), and optional `recommendation`. Long content and more than a few options degrade usability: cramped layout, insufficient space for per-option commentary, and weak visual hierarchy between context, options, and recommendation.

## Goals

### G1 — Layout and scrolling

- Primary **scrollable** region for `summary` + `context_markdown` + per-option detail, with a **visible viewport** that does not grow unbounded in height.
- **Sticky** or always-visible primary actions (select + confirm / cancel) so long markdown does not push controls below the fold on common laptop viewports.

### G2 — Options list

- Render each option as a **card** or **accordion row**: `label` as title; `detail_markdown` in expandable body (default collapsed when more than N characters or when option count > 3).
- Support **many options** (target at least 10): either virtualized list, pagination (“Show more”), or two-column layout on wide screens with graceful single-column on narrow.
- Clear **selected** state before submit.

### G3 — Recommendation

- Distinct **visual treatment** for `recommendation` (callout, badge, or linked highlight on the matching option card).
- Optional **one-click “Use recommendation”** that selects the matching `option_id` (still submits canonical `id` as `answer`).

### G4 — Markdown safety and rendering

- Render markdown for `context_markdown` and `detail_markdown` with the same safety posture as today (sanitized subset or plain-text fallback). Document allowed constructs in this file or `skill/ailoop/references/ailoop-api.md`.

### G5 — Accessibility and density

- Keyboard navigable option list; focus order enters expandable details sensibly.
- Sufficient contrast and touch targets for mobile operators.

## Non-goals

- Redesigning non-decision message types.
- Rich WYSIWYG editing of responses (human still picks an option id).

## Acceptance criteria

1. Fixture or manual checklist: decision with **long** `context_markdown` and **8+** options remains usable without horizontal scroll on 1280px width.
2. Each option’s `detail_markdown` readable without forcing all options open simultaneously (accordion or equivalent).
3. Recommendation visibly distinguishable; selecting recommended option produces correct `answer` id in outbound response.
4. Screenshots or short screen recording linked from PR optional; at minimum QA steps documented in PR description.

## References

- Wire: `ailoop-core` `MessageContent::Decision`.
- Server TTY reference: `ailoop-server/src/server/core.rs` decision handling for parity of semantics (id vs label vs index).
