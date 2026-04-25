# javalens-manager

Desktop manager for running and orchestrating JavaLens MCP servers across multiple Java projects.

`javalens-manager` provides a clean desktop experience for managing `javalens-mcp` instances, project workspaces, runtime state, and MCP client setup. By default it pulls the runtime from a maintained fork ([hw1964/javalens-mcp](https://github.com/hw1964/javalens-mcp)) that ships fixes upstream has not picked up yet — see *Release source* in the Settings UI to switch sources. The manager itself remains a pure orchestrator.

## Status

**Beta (v0.10.1)**: `javalens-manager` is a fully functional desktop application on Linux. It supports registering multiple Java projects, automatically downloading and managing JavaLens runtimes, and deploying MCP configurations directly to Cursor, Claude Desktop, Antigravity, and IntelliJ. While feature-complete for Linux, broader OS support and QA testing are ongoing before a stable 1.0 release.

### What's new in v0.10.1

- **Icon refresh.** The white outer ring around the magnifying glass is now a very dark navy (`#0c1838`) so the icon sits cleanly inside dark dock/taskbar themes.
- **Discover and Import-selected button states.** After Discover finishes, the button greys out (re-clicking the same path is a no-op). After a successful workspace import, the form resets — workspace file, candidates, and selection all clear so the form is ready for the next operation.

### What's new in v0.10.0

- **Release source is configurable.** Settings → JavaLens Runtime → *Release source* lets you switch between the maintained fork (default), the original upstream, or a custom GitHub repo. Switching the dropdown auto-saves and pulls the new repo's latest jar in one click.
- **Source-resolution fix shipped via the fork.** `javalens-mcp` 1.2.1 (in the fork) honors Maven `<sourceDirectory>` / `<testSourceDirectory>` overrides and Eclipse `.classpath` `kind="src"` / `kind="lib"` entries. Hybrid Maven+PDE projects and non-conventional Eclipse layouts are now indexed correctly.
- **Settings UI decluttered.** The Runtime panel drops the always-visible status chips and Refresh button; auto-update on dashboard load covers refreshes, and a Download button appears only when an update is actually available.
- **Bug fix: project list refresh.** Adding a second project via the form no longer lands on a stale port; the form re-arms the suggested port after each successful save. Stale "Unknown project id" errors after deletion are also gone.
- **Polish.** Equal-height Settings panels, full-width Discover button, primary-blue button color only when the action is actually clickable.

## Docs

- [`docs/plan.md`](docs/plan.md) - project scope, boundaries, and core decisions
- [`docs/roadmap.md`](docs/roadmap.md) - sprint-level roadmap for this repository
- [`docs/architecture.md`](docs/architecture.md) - architecture direction and module boundaries
- [`docs/adr/README.md`](docs/adr/README.md) - accepted architecture decision records
- [`docs/javalens-management.md`](docs/javalens-management.md) - how upstream `javalens-mcp` is imported, versioned, and managed
- [`docs/javalens-runtime-contract.md`](docs/javalens-runtime-contract.md) - exact Sprint 1 launch, transport, health, and config contract
- [`docs/sprint-1-backlog.md`](docs/sprint-1-backlog.md) - first runnable-slice backlog and acceptance criteria
- [`docs/sprint-2-backlog.md`](docs/sprint-2-backlog.md) - managed-runtime upgrade backlog, team split, and acceptance criteria
- [`docs/sprint-5-backlog.md`](docs/sprint-5-backlog.md) - paths, workspace override, and Dashboard vs Settings UX documentation
- [`docs/sprint-6-backlog.md`](docs/sprint-6-backlog.md) - radical simplification and global configuration
- [`docs/tauri-bootstrap.md`](docs/tauri-bootstrap.md) - Sprint 0 and Sprint 1 bootstrap path for the Tauri app

## Installation

You can install or update `javalens-manager` on Linux using the provided installation script. This script will download the latest `.AppImage` and set up a desktop entry for you.

Run the following command in your terminal:

```bash
curl -sSL https://raw.githubusercontent.com/hw1964/javalens-manager/main/install.sh | bash
```

Alternatively, you can download the `.deb` or `.AppImage` files manually from the [GitHub Releases page](https://github.com/hw1964/javalens-manager/releases).

If you launch the `.AppImage` manually, ensure it has executable permission first:

```bash
chmod +x javalens-manager_0.10.1_amd64.AppImage
./javalens-manager_0.10.1_amd64.AppImage
```

## What It Is

`javalens-manager` is a desktop application for:

- registering Java projects
- starting and stopping `javalens-mcp` instances
- managing per-project runtime state and workspaces
- showing health and status for running servers
- helping MCP clients connect to the right `javalens` instance
- reducing friction when using JavaLens across more than one project

## What It Is Not

`javalens-manager` is not:

- a fork of `javalens-mcp`
- a replacement for `javalens-mcp`
- an Eclipse plugin
- a Java code analysis engine by itself

Java semantic analysis and refactoring remain the responsibility of upstream `javalens-mcp`.

## Why This Exists

`javalens-mcp` is a strong semantic Java MCP server built on Eclipse JDT, but it is centered on a single running server/session and project-loading workflow.

This project exists to provide a higher-level desktop experience for people who want to use JavaLens across multiple Java projects without manually managing processes, workspaces, configuration, and client setup.

## Goals

- Provide a modern desktop UI for JavaLens operations
- Keep upstream `javalens-mcp` completely unchanged
- Support multiple Java projects through managed JavaLens instances
- Make project registration and runtime management simple
- Improve day-to-day usability for MCP-based Java workflows
- Work well with Cursor, VS Code, Claude Code, and other MCP-capable clients

## Non-Goals

- Reimplement Java semantic analysis
- Add custom Java refactoring logic outside JavaLens
- Replace IDE-native Java tooling
- Bundle proprietary project-specific integrations into this repository

## Planned Features

- project registry
- start / stop / restart controls per project
- health and status display
- log viewing
- workspace and runtime management
- generated MCP client configuration where useful
- system tray integration
- preferences for ports, paths, and startup behavior

## Architecture Direction

`javalens-manager` is planned as a Tauri desktop application with:

- a Rust backend for process/runtime orchestration
- a desktop UI for project and server management
- integration with upstream `javalens-mcp` as an external dependency
- clear separation between UI, configuration, process management, and MCP client setup

## Relationship to JavaLens

This project is built around upstream [`javalens-mcp`](https://github.com/pzalutski-pixel/javalens-mcp).

Important design rule:

- `javalens-manager` manages `javalens-mcp`
- `javalens-manager` does not modify `javalens-mcp`

If you need semantic Java analysis, navigation, refactoring, or diagnostics, those capabilities come from JavaLens itself.

## Roadmap

### Current Focus
- Cross-platform testing (macOS, Windows)
- Broader QA and edge-case testing
- Automated updates and binary patching

### Completed
- **Sprint 8:** Packaging and Distribution (Automated GitHub Releases, Linux `.deb` and `.AppImage` installers, auto-update UX)
- **Sprint 6:** Radical simplification and global configuration
- **Sprint 5:** Paths, workspace overrides, and Dashboard vs Settings UX
- **Sprint 2:** Managed-runtime upgrades and team split
- **Sprint 1:** First runnable slice (Project registry, start/stop controls, health display)

## Tech Stack

Planned stack:

- Tauri
- Rust
- desktop frontend UI
- upstream `javalens-mcp`

## Development

Project setup instructions now start in [`docs/tauri-bootstrap.md`](docs/tauri-bootstrap.md).

Expected local prerequisites will include:

- Rust toolchain
- Tauri prerequisites
- Java 21+
- access to `javalens-mcp`

## License

MIT

This project is intended to stay permissively licensed and compatible with upstream `javalens-mcp`.
