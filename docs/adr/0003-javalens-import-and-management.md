# ADR-0003: JavaLens Import and Management

- Status: Accepted
- Date: 2026-04-19

## Context

`javalens-manager` depends on upstream `javalens-mcp`, but the repository purpose and current docs are clear that upstream must remain unchanged.

The manager still needs a concrete operating model for:

- how upstream `javalens-mcp` is supplied
- where manager-owned state lives
- how runtimes are launched and supervised
- how MCP-capable clients discover the right server connection

## Decision

`javalens-manager` will consume upstream `javalens-mcp` as an unchanged external runtime artifact and manage it from outside the project source trees.

### Dependency model

- upstream `javalens-mcp` is treated as a black-box dependency
- the manager does not vendor modified source, fork the project, or embed custom semantic logic
- the manager should support selecting a specific upstream release artifact or local JAR path for development and troubleshooting
- pinned versions should be recorded in manager configuration, not hidden in project-specific source changes

### Runtime ownership

For each registered project, the manager owns:

- runtime launch parameters
- per-project workspace/data paths
- process lifecycle and supervision
- logs, health state, and recovery metadata
- generated MCP client connection details

Upstream `javalens-mcp` owns:

- Java semantic analysis
- refactoring behavior
- diagnostics and tool responses

### Filesystem strategy

Manager-owned data must live outside registered project folders.

Default categories:

- config: project registry, preferences, pinned upstream version/path
- state: running instance metadata, generated connection details
- logs: per-project runtime logs
- cache/tools: downloaded or staged upstream runtime artifacts when the manager later adds artifact acquisition

On Linux, prefer an XDG-style layout under the user's home directory. Equivalent platform-native locations can be used on Windows and macOS.

### MCP client configuration

Generated MCP client configuration is an adapter owned by the manager. It should be derived from the active project runtime and written in a form that external clients can consume without making it part of the project's source of truth.

## Consequences

Positive:

- upstream upgrades stay straightforward
- project repositories stay clean
- runtime issues can be diagnosed without mixing manager state into source trees
- client setup becomes reproducible and automatable

Negative:

- the manager must own version/path validation and artifact discovery UX
- installation/update flows for upstream artifacts need to be documented carefully
- generated client config can drift if runtime state changes and the manager does not refresh it consistently

## Alternatives Considered

### Fork and bundle a modified `javalens-mcp`

Rejected because it would blur responsibilities, complicate licensing and upgrades, and fight the stated goal of keeping upstream untouched.

### Store runtime data inside each Java project

Rejected because it pollutes project trees, makes source control noisier, and weakens the manager's role as the owner of operational state.
