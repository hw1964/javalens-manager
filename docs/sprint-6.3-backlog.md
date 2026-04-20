# Sprint 6.3 Backlog (Settings Enhancements)

## Goal

Keep manager state clean over long-running usage by adding safe cleanup actions in Settings.

## Stories

### 1) Clean/Delete Project Logs

Acceptance criteria:
- Add a settings action to clean all runtime log files produced by managed projects.
- Show before/after feedback (for example: number of files removed).
- Require confirmation before destructive delete.
- Continue to work when some log files are missing or locked (best-effort with clear warning).

### 2) Clean/Delete Project Workspaces

Acceptance criteria:
- Add a settings action to clean all manager-owned workspace directories used for indexing/cache.
- Require confirmation before destructive delete.
- Preserve manager settings and project registration metadata.
- Report partial failures without aborting full cleanup.

### 3) Combined Cleanup Action

Acceptance criteria:
- Add an optional one-click "Clean all generated data" action for logs + workspaces.
- Show a clear warning describing exactly what will be deleted.
- Provide success/partial-failure summary.

### 4) Safety and UX

Acceptance criteria:
- Disable cleanup actions while conflicting operations are running.
- Surface cleanup errors in concise, user-readable messages.
- Keep cleanup controls in Settings only (not Dashboard).
