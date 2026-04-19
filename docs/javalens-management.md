# JavaLens Import and Management

## Purpose

This document explains how `javalens-manager` should consume and operate upstream `javalens-mcp` without modifying it.

The short version is:

- `javalens-manager` manages `javalens-mcp`
- `javalens-manager` does not fork, patch, or reimplement `javalens-mcp`
- all operational state belongs to the manager, not to the Java project repositories

## Ownership Boundary

### Upstream `javalens-mcp` owns

- Java semantic analysis
- refactorings
- diagnostics
- MCP tool behavior and responses

### `javalens-manager` owns

- project registration
- runtime start / stop / restart
- process supervision
- workspace and runtime directory mapping
- logs and health reporting
- client connection details
- generated MCP client configuration where useful

## How JavaLens Should Be Supplied

Treat upstream `javalens-mcp` as an external runtime artifact.

Recommended support order:

1. manager-owned cached runtime downloaded from upstream releases
2. pinned upstream release selection in manager settings
3. user-provided local JAR path as an advanced fallback

This keeps the normal path simple for users while preserving a clean fallback for development and troubleshooting.

## Versioning Rule

The manager should record which `javalens-mcp` artifact each runtime uses.

That version or artifact path should be stored in manager-owned configuration, not hidden inside project folders or mixed into application code. This makes upgrades, downgrades, and troubleshooting explicit.

The manager should distinguish between:

- managed runtime selection by version from the local cache
- local JAR fallback selection for advanced/manual use

## Filesystem Layout

Manager state should live outside project source trees.

Recommended logical layout:

- config: project registry, preferences, selected upstream artifact/version
- state: active runtime metadata, generated connection details
- logs: per-project runtime logs
- cache/tools: staged upstream artifacts when artifact download support is added
- workspaces: per-project `javalens-mcp` workspace/data directories

### Linux default shape

Use XDG-style directories under the user home directory:

```text
~/.config/javalens-manager/
  projects.json
  settings.json

~/.local/state/javalens-manager/
  runtimes/
  logs/
  generated/

~/.cache/javalens-manager/
  tools/javalens/     # managed JavaLens unpacks (versioned dirs); configurable via settings dataRoot
  workspaces/         # per-project Eclipse/JDT -data dirs
```

The exact file names can evolve, but the categories should remain stable.

## Runtime Contract Per Project

Each registered project maps to the global runtime definition. The manager tracks:

- project path
- display name
- assigned port or connection settings
- workspace path (derived from data root)
- log path
- current status and last-known health

This should be serializable so the manager can restore state and present clear diagnostics.

## Launch Model

For each project, the manager should:

1. resolve the configured upstream artifact
2. resolve or create the managed workspace path
3. resolve the runtime port and environment
4. spawn the `javalens-mcp` process
5. capture stdout/stderr into manager-owned logs
6. expose health and connection data to the UI

The launch code should treat `javalens-mcp` as a black-box process with a stable contract, not as embedded business logic.

## Logging and Diagnostics

Logs must remain manager-owned and per runtime.

Minimum expectations:

- separate log file or stream per project runtime
- last start time, last stop time, and exit status
- clear surface for launch failures such as missing Java, missing JAR, invalid path, or port collision

This is essential because the value of `javalens-manager` is operational clarity around an external runtime.

## Generated MCP Client Configuration

Generated MCP configuration should be treated as an adapter layer, not as the authoritative source of runtime truth.

Recommended behavior:

- derive generated config from the active runtime record
- keep a manager-owned copy of generated connection details
- optionally write client-specific config files only when the user asks or enables it
- never require manual editing inside project source trees as the primary control path

## Update Policy

The manager should persist an update policy for managed JavaLens runtimes:

- `always`: keep the latest upstream release cached automatically
- `ask`: detect updates and let the user approve them before download

If no managed runtime is present yet, the manager may still bootstrap the latest release so the app has a usable default runtime.

## Update Strategy

When a new upstream `javalens-mcp` release is adopted:

1. record the new version/artifact in manager config
2. validate it against at least one managed runtime
3. allow per-project rollback if needed
4. refresh generated client config if connection details changed

This keeps updates explicit and reversible.

## Non-Goals

- building a custom fork of `javalens-mcp`
- storing runtime state inside user project repositories
- mixing UI code with artifact/process supervision logic
- assuming one `javalens-mcp` instance can safely serve unrelated projects
