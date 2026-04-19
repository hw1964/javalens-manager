# ADR-0002: Runtime and Process Model

- Status: Accepted
- Date: 2026-04-19

## Context

Upstream `javalens-mcp` is oriented around a single loaded project and running server/session. The existing project docs already assume that multi-project support in `javalens-manager` comes from managing multiple isolated runtimes instead of turning one server into a shared multi-project process.

The manager therefore needs a runtime model that is simple to supervise, debug, and expose in the UI.

## Decision

`javalens-manager` will use one isolated `javalens-mcp` runtime per registered project.

Each managed project has:

- one project registry entry
- one runtime definition
- one workspace/data directory
- one status and health stream
- one lifecycle surface for start, stop, restart, and recovery

Multi-project behavior is achieved by repeating this model cleanly for multiple projects. The manager will not attempt to make one `javalens-mcp` process serve many unrelated projects at once.

## Consequences

Positive:

- process ownership is explicit and easy to debug
- failures are isolated to one project runtime
- logs, ports, and health state map cleanly to one project
- the UI can present a predictable operational model

Negative:

- multiple projects may consume more memory and ports
- startup work is repeated per project
- bulk actions such as "start all" require orchestration over several runtimes

## Operational Notes

- runtime supervision should be independent from UI code
- registry, workspace, and process metadata should stay serializable and testable
- health checks and restart behavior should be implemented per runtime first before adding convenience features such as global controls

## Alternatives Considered

### Single shared process for all projects

Rejected because it does not match the upstream model well and would complicate process isolation, failure handling, workspace mapping, and UI state.
