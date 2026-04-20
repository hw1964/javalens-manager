# Sprint 8 Backlog

## Goal

Ship and distribute the complete javalens-manager application with installable executables, install/update scripts, and automated GitHub Release publishing.

## Problem Statement

- The app is runnable in development, but deployment workflow is not yet productized.
- Users need an installable package and a repeatable upgrade path.
- Release publishing should be automated and reproducible, not manual-only.

## Stories

### 1. Packaging Targets and Build Artifacts

Acceptance criteria:
- Define supported deployment targets and artifact types (minimum: Linux desktop package + standalone executable).
- Produce consistent versioned artifacts per release.
- Include checksum files for all release artifacts.
- Validate artifact startup on clean test environments.

### 2. Installer and Update Script

Acceptance criteria:
- Provide install script(s) that:
  - download release assets
  - verify checksum/signature policy
  - install binaries/assets into expected paths
  - create desktop entry/launcher where applicable
- Provide update script with safe in-place upgrade and rollback guidance.
- Provide uninstall script or documented uninstall procedure.

### 3. GitHub Release Automation

Acceptance criteria:
- Add release workflow that builds artifacts from tags/releases and uploads them to GitHub Releases.
- Publish release notes template with:
  - version
  - supported platforms
  - install/update instructions
  - known limitations
- Fail release on missing artifacts/checksums/validation failures.

### 4. Runtime/Config Migration Safety

Acceptance criteria:
- Validate compatibility/migration behavior for existing config/state directories.
- Preserve user settings and projects across upgrades.
- Add backup or migration guard for incompatible schema changes.

### 5. Deployment UX in App

Acceptance criteria:
- Add an in-app "How to Install/Update" entry (link/panel) pointing to release assets and instructions.
- Add version/build visibility and channel info (stable/dev) in About.
- Show update availability status tied to release metadata.

### 6. Help, About, and Licensing

Acceptance criteria:
- Add Help action that opens bundled product help PDF.
- Add About dialog with:
  - app version/build
  - runtime version
  - license summary
  - link/path to full license text
- Ensure license text is included in distributed artifacts.

### 7. Documentation

Acceptance criteria:
- Add deployment runbook for maintainers:
  - preparing a release
  - validating artifacts
  - publishing and rollback
- Add user-facing install/update quickstart docs.
- Document platform-specific caveats and troubleshooting.

## Team Split

- `release-engineer`: CI/CD release workflow, artifact validation, GitHub publishing.
- `tauri-engineer`: packaging config, runtime migration safety, installer hooks.
- `frontend-engineer`: About/Help/update surfaces in app.
- `qa-test-engineer`: install/update/uninstall matrix and rollback verification.
- `docs-engineer`: release runbook and end-user install/update docs.

## Deferred

- Delta/binary patch updates.
- Auto-update service integration beyond scripted installer/update flow.
- Enterprise deployment channels (MSI repos, apt/yum repos, managed MDM integration).
