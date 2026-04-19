# Roadmap

## Current Position

This repository is in the planning and bootstrap stage. The first goal is to turn the repo into a clean, documented starting point before scaffolding the Tauri application.

## Sprint 0

Focus on planning, boundaries, and setup.

Primary Sprint 0 references:

- [`docs/adr/README.md`](adr/README.md)
- [`docs/javalens-management.md`](javalens-management.md)
- [`docs/tauri-bootstrap.md`](tauri-bootstrap.md)

Sprint 0 deliverables:

- confirm repo structure and workspace setup using [`docs/tauri-bootstrap.md`](tauri-bootstrap.md)
- document scope, non-goals, and architecture direction in the core docs
- adopt the first ADRs in [`docs/adr/README.md`](adr/README.md)
- confirm Rust and Tauri prerequisites using [`docs/tauri-bootstrap.md`](tauri-bootstrap.md)
- lock the initial frontend choice in [`docs/adr/0004-tauri-frontend-choice.md`](adr/0004-tauri-frontend-choice.md)
- define the first thin vertical slice in [`docs/tauri-bootstrap.md`](tauri-bootstrap.md)

## Sprint 1

Focus on the first working manager slice.

- create the Tauri application skeleton
- add configuration model and project registry
- add process lifecycle handling for one `javalens` instance
- add a minimal status view
- verify start, stop, restart, and health-check flow

## Sprint 2

Focus on hardening the manager into a useful daily tool.

- improve logs and error visibility
- add per-project workspace/runtime settings
- add generated MCP client configuration where useful
- improve recovery behavior and operational UX
- add focused tests around lifecycle and configuration

## Later Milestones

- system tray integration
- stronger multi-project workflows
- better onboarding for MCP-capable clients
- richer health diagnostics and troubleshooting

## Not On The Immediate Roadmap

- custom Java semantic features outside upstream `javalens-mcp`
- heavy optimization of agent MCP usage before the runtime is stable
