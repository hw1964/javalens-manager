# Sprint 7 Backlog

## Goal

Deploy project-specific JavaLens MCP services into client MCP configs automatically (Cursor, Claude, Antigravity, IntelliJ), and generate client-specific MCP-first rule blocks that enforce "MCP tools before grep/find/manual refactor" behavior.

## Problem Statement

- Today, each MCP client has its own config source.
- Registering servers in one client does not automatically register them in another.
- This creates duplicated setup work and drift between clients.

## Stories

### 1. MCP Config Generation Layer

Acceptance criteria:
- Add a generator service that produces client-specific MCP config payloads from manager project state.
- Generated entries include stable server names, launch command, arguments, env, and project-specific runtime metadata/services.
- Generation supports:
  - Cursor
  - Claude
  - Antigravity
  - IntelliJ
- Generated config contains one managed section per client with all currently selected project services.

### 2. Export Targets and Update Policy

Acceptance criteria:
- Support writing generated configs to user-chosen export locations (no hidden destructive overwrite).
- Support preview before write.
- Support update modes:
  - replace managed section only
  - append missing entries
- Preserve non-manager-managed config sections.

### 3. Deploy UX (Dashboard + Menu)

Acceptance criteria:
- Add a primary deploy trigger in Dashboard (`Deploy to Agents`) and a matching menu action.
- Deploy action generates and writes both MCP config and MCP-first rules for selected clients in one flow.
- Provide preview/dry-run mode before write and post-deploy per-client status summary.
- Show last deployment timestamp and per-client result state (success/fail/skipped).
- Define deploy action semantics explicitly:
  - `Deploy to Agents`: normal action (generate + write configs/rules).
  - `Dry run`: simulate deploy, validate, and show what would change without writing files.
  - `Preview`: show generated content/diff before writing.
  - `Regenerate`: force rewrite of manager-owned managed sections, even if unchanged.

### 4. Settings Integration

Acceptance criteria:
- Keep a "Client MCP Integration" section in Settings for target selection, path overrides, merge mode, and backup policy.
- Allow selecting target clients and output paths.
- Reuse Sprint 6.3 Settings metadata (auto-detected defaults, manual overrides, merge/backup policy flags) as deploy input.
- Settings does not replace Dashboard/Menu deploy trigger ownership.

### 5. System Tray Close Behavior

Acceptance criteria:
- If `useSystemTray = true` and managed services are running, window close does not stop services; app hides/minimizes to tray.
- Minimize action remains normal OS minimize behavior (taskbar/dock), not forced tray-hide.
- If `useSystemTray = false`, close follows normal app close behavior.
- Tray menu includes at least: `Show`, `Stop all services`, `Quit`.
- `Quit` performs explicit shutdown flow (with confirmation when services are running, per product policy).

### 6. Validation and Safety

Acceptance criteria:
- Validate generated configs against expected schema shape before writing.
- Validate referenced runtime paths exist.
- Validate generated rule references against generated MCP server ids/targets.
- Warn when project runtime is unresolved or service metadata is incomplete for selected targets.
- Add dry-run diagnostics for permission/path errors.

### 7. Documentation

Acceptance criteria:
- Add a short operational guide describing how each client loads MCP config.
- Document deploy/regenerate workflow when projects are added/removed/renamed.
- Document limitations and fallback manual setup.

### 8. Agent Rules Generation

Acceptance criteria:
- Generate rule/policy artifacts for Cursor, Claude, Antigravity, and IntelliJ from manager project state.
- Rule content explicitly enforces:
  - MCP service/tool calls first when capability exists
  - filesystem `grep/find` or manual refactor only as fallback
- Support managed markers/blocks so redeploy updates only manager-owned rule sections.
- Expose deploy/redeploy lifecycle in UI and docs (when to rerun, overwrite behavior, conflict handling).

### 9. Rule Validation

Acceptance criteria:
- Validate generated rule syntax/shape before writing.
- Validate referenced server ids and command targets exist in generated MCP config.
- Surface per-client validation errors clearly without writing partial invalid output.

## Team Split

- `platform-architect`: define generator interfaces and managed section merge strategy.
- `tauri-engineer`: implement Rust generation/deploy commands and file safety checks.
- `frontend-engineer`: dashboard/menu deploy triggers plus settings target/path UX.
- `qa-test-engineer`: cross-client validation matrix and regression tests for deploy/merge/write logic.
- `agent-integration-engineer`: map per-client agent rule formats and MCP-first policy templates for all four clients.

## Deferred

- none
