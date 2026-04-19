# javalens-manager

Desktop manager for running and orchestrating JavaLens MCP servers across multiple Java projects.

`javalens-manager` provides a clean desktop experience for managing upstream `javalens-mcp` instances, project workspaces, runtime state, and MCP client setup. It does not fork or modify `javalens` itself.

## Status

Early-stage project. Architecture and implementation are in progress.

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

### Phase 1
- Create the desktop manager foundation
- Register projects
- Launch and stop JavaLens instances
- Show runtime status and health
- Support one managed JavaLens instance per project

### Later
- Multi-project operational improvements
- richer client setup flows
- stronger UX around logs, status, and recovery
- deeper integration patterns for editor and agent workflows

## Tech Stack

Planned stack:

- Tauri
- Rust
- desktop frontend UI
- upstream `javalens-mcp`

## Development

Project setup instructions will be added as implementation begins.

Expected local prerequisites will include:

- Rust toolchain
- Tauri prerequisites
- Java 21+
- access to `javalens-mcp`

## License

MIT

This project is intended to stay permissively licensed and compatible with upstream `javalens-mcp`.
