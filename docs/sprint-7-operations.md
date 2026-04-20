# Sprint 7 Operations Guide

## Purpose

Sprint 7 introduces deploy automation for client MCP config + MCP-first rule artifacts across:

- Cursor
- Claude
- Antigravity
- IntelliJ

Deploy modes:

- `Deploy to Agents` (write)
- `Dry run` (validate without write)
- `Preview` (show generated content intent)
- `Regenerate` (force rewrite managed sections)

## Preconditions

- At least one managed project is configured.
- MCP client paths are configured in Settings (`MCP Config Locations`).
- JavaLens runtime path resolves for managed projects.

## Deploy Workflow

1. Open Dashboard.
2. Trigger one of the deploy actions.
3. Review per-client result summary.
4. For failures, correct path/runtime issues in Settings, then rerun.

## Managed Section Behavior

- The manager writes only manager-owned managed sections.
- Non-manager sections remain preserved.
- Merge behavior follows `mcpMergeMode`.
- Backups are written when `mcpBackupBeforeWrite` is enabled.

## Tray Close Behavior

- If `useSystemTray=true` and services are running: close hides to tray.
- Minimize remains normal OS minimize behavior.
- Tray menu supports `Show`, `Stop all services`, `Quit`.

## Troubleshooting

- **Path validation failed**: ensure target config parent directory exists.
- **No deployable services**: verify project runtime resolves and project list is non-empty.
- **Partial client failures**: inspect per-client `validationErrors` and rerun.
