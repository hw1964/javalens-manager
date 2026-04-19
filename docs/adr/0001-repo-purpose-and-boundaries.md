# ADR-0001: Repo Purpose and Boundaries

- Status: Accepted
- Date: 2026-04-19

## Context

`javalens-manager` is a dedicated repository for a desktop manager/runtime shell around upstream `javalens-mcp`.

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
- absorb unrelated tooling that does not belong to the JavaLens manager

The repository boundary should stay narrow enough that the desktop manager can evolve without mixing in broader tooling concerns.

## Consequences

Positive:

- repository scope stays small and easier to reason about
- upstream compatibility remains straightforward
- architecture and backlog stay aligned with the actual product boundary

Negative:

- some future ideas may need their own repository or their own documentation set
- the boundary needs to be defended when adjacent tooling ideas appear

## Follow-On Implications

- future docs should treat `javalens-manager` as a manager/orchestrator, not as a Java tooling engine
- upstream `javalens-mcp` should always be referenced as an external dependency
- future documents in this repository should stay inside the manager/runtime boundary
