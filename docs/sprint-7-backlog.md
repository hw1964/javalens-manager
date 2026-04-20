# Sprint 7 Backlog

## Goal

Make managed JavaLens servers discoverable across clients by generating MCP client configuration artifacts automatically (Cursor, Claude, Antigravity), and generate client-specific agent rule blocks that enforce MCP-service-first behavior over ad-hoc filesystem search flows.

## Problem Statement

- Today, each MCP client has its own config source.
- Registering servers in one client does not automatically register them in another.
- This creates duplicated setup work and drift between clients.

## Stories

### 1. MCP Config Generation Layer

Acceptance criteria:
- Add a generator service that produces client-specific MCP config payloads from manager project state.
- Generated entries include stable server names, launch command, arguments, env, and per-project runtime metadata.
- Generation supports at least:
  - Cursor
  - Claude
  - Antigravity

### 2. Export Targets and Update Policy

Acceptance criteria:
- Support writing generated configs to user-chosen export locations (no hidden destructive overwrite).
- Support preview before write.
- Support update modes:
  - replace managed section only
  - append missing entries
- Preserve non-manager-managed config sections.

### 3. Dashboard and Settings UX

Acceptance criteria:
- Add a "Client MCP Integration" section in Settings.
- Allow selecting target clients and output paths.
- Provide "Generate" and "Regenerate" actions with success/error feedback.
- Show last generation timestamp and per-client status.
- Reuse Sprint 6.3 Settings metadata (auto-detected defaults, manual overrides, merge/backup policy flags) as input; deploy execution remains owned here in Sprint 7.

### 4. Validation and Safety

Acceptance criteria:
- Validate generated configs against expected schema shape before writing.
- Validate referenced runtime paths exist.
- Warn when project runtime is unresolved or not running (if required by target client workflow).
- Add dry-run diagnostics for permission/path errors.

### 5. Documentation

Acceptance criteria:
- Add a short operational guide describing how each client loads MCP config.
- Document regeneration workflow when projects are added/removed/renamed.
- Document limitations and fallback manual setup.

### 6. Agent Rules Generation

Acceptance criteria:
- Generate rule/policy artifacts for Cursor, Claude, and Antigravity from manager project state.
- Rule content explicitly prefers MCP service/tool calls when capability exists, with filesystem `grep/find` as fallback only.
- Support managed markers/blocks so regenerate updates only manager-owned rule sections.
- Expose regenerate lifecycle in UI and docs (when to rerun, overwrite behavior, conflict handling).

### 7. Rule Validation

Acceptance criteria:
- Validate generated rule syntax/shape before writing.
- Validate referenced server ids and command targets exist in generated MCP config.
- Surface per-client validation errors clearly without writing partial invalid output.

## Team Split

- `platform-architect`: define generator interfaces and managed section merge strategy.
- `tauri-engineer`: implement Rust generation/export commands and file safety checks.
- `frontend-engineer`: settings UI for client targets, preview, and generate actions.
- `qa-test-engineer`: cross-client validation matrix and regression tests for config merge/write logic.
- `agent-integration-engineer`: map per-client agent rule formats and MCP-first policy templates.

## Deferred

- Auto-reload/restart of external clients after config generation.
- Cloud sync of generated MCP config.
- One-click install of external client binaries.
