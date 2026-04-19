# ADR-0001: Repo Purpose and Boundaries

- Status: Accepted
- Date: 2026-04-19

## Context

`javalens-manager` is the Phase 1 repository in a two-repo program:

- `javalens-manager` in `~/CursorProjects` provides the desktop manager/runtime shell
- `eclipse-ai-plugin` in `~/Projects` is a separate Eclipse plugin project

The existing project docs already establish that this repository owns orchestration, runtime supervision, project registration, workspace management, and MCP client setup. They also state that upstream `javalens-mcp` remains unchanged and that Java semantic behavior stays upstream.

Without a formal ADR, those decisions are repeated across `README.md`, `docs/plan.md`, and `docs/architecture.md`, which risks drift as implementation starts.

## Decision

`javalens-manager` is the dedicated Phase 1 repository for managing upstream `javalens-mcp` instances across multiple Java projects.

This repository will:

- manage project registration and local project metadata
- start, stop, restart, and supervise `javalens-mcp` runtimes
- own desktop UX, status, logs, health checks, and recovery flows
- manage per-project workspace/runtime directories
- generate MCP client configuration where useful

This repository will not:

- fork, patch, or reimplement upstream `javalens-mcp`
- embed custom Java semantic analysis or refactoring logic
- implement the Eclipse plugin
- include proprietary JATS integration

The Eclipse plugin remains a separate repository with its own lifecycle, backlog, and release process.

## Consequences

Positive:

- repository scope stays small and easier to reason about
- upstream compatibility remains straightforward
- Phase 1 and Phase 2 can evolve independently
- architecture and backlog stay aligned with the actual product boundary

Negative:

- some concepts and docs will be duplicated across repos at first
- cross-repo integration points must be documented explicitly
- shared abstractions cannot be assumed to live in one monorepo

## Follow-On Implications

- future docs should treat `javalens-manager` as a manager/orchestrator, not as a Java tooling engine
- upstream `javalens-mcp` should always be referenced as an external dependency
- any Phase 2 or JATS-specific material belongs outside this repository unless it is cited only as external context
