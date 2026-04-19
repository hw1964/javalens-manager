# JavaLens Runtime Contract

## Purpose

This document freezes the first-slice runtime contract for launching upstream `javalens-mcp` from `javalens-manager`.

It turns the general guidance from `docs/javalens-management.md` into an implementation contract for Sprint 1.

## Source Basis

The contract is based on the upstream `javalens-mcp` README:

- launch via `java -jar /path/to/javalens.jar -data /path/to/workspace`
- transport is MCP over stdio
- optional auto-load via `JAVA_PROJECT_PATH`
- readiness can be checked with the upstream `health_check` tool

## Exact Launch Contract

For the first slice, the manager launches JavaLens as:

```bash
java -jar /path/to/javalens/javalens.jar -data /path/to/manager-owned/workspace
```

With environment:

- `JAVA_PROJECT_PATH=/path/to/java/project`

Manager rules:

- the source project path stays untouched
- the workspace path is manager-owned and lives outside the source tree
- stdout and stderr are written to a manager-owned log file
- stdin remains attached because upstream MCP uses stdio

## Transport Decision

The Phase 1 runtime slice is explicitly **stdio-based**, not port-based.

That means:

- there is no port field in the first runnable slice
- the UI should surface transport, workspace, process state, and log location
- generated external MCP client config remains deferred until after the one-project lifecycle is stable

## Health Semantics For Sprint 1

The first slice uses a two-level health model:

1. **Process liveness** is the implemented health signal in this repo now.
2. **Upstream MCP `health_check`** is the next-level semantic health probe, but is deferred until the manager speaks to the child process over stdio.

Operational meaning in Sprint 1:

- `starting`: process spawned recently and has not exited
- `running`: process is still alive after the short warm-up window
- `stopped`: process is not running
- `failed`: process exited unsuccessfully or could not be started

This is intentionally narrow and honest. It gives a reliable first vertical slice without pretending that process liveness is the same as full MCP readiness.

## Stable Config Schema

For Sprint 1, `projects.json` uses this schema shape:

```json
{
  "version": 1,
  "projects": [
    {
      "id": "example-service-1713550000000",
      "name": "Example Service",
      "projectPath": "/path/to/java/project",
      "javalensJarPath": "/path/to/javalens.jar",
      "workspaceDir": "/home/user/.cache/javalens-manager/workspaces/example-service-1713550000000"
    }
  ]
}
```

Field meanings:

- `id`: stable manager-owned identifier
- `name`: user-facing display name
- `projectPath`: Java project root to auto-load into JavaLens
- `javalensJarPath`: selected upstream runtime artifact
- `workspaceDir`: manager-owned Eclipse/JDT workspace path passed to `-data`

## Runtime-Owned Files

For each project runtime, the manager owns:

- one config entry in `projects.json`
- one workspace directory
- one log file under the manager state directory
- one in-memory runtime handle while the child process is alive

## Explicitly Deferred

These stay out of the first slice:

- MCP proxying for external clients
- generated `.vscode/mcp.json` flows
- semantic readiness based on upstream `health_check`
- multi-project batch orchestration
- tray/process background behavior
