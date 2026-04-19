# Project Plan

## Purpose

`javalens-manager` is a Tauri desktop manager for upstream `javalens-mcp`.

This repository owns the manager/runtime side only and is intentionally scoped to the desktop application, runtime supervision, and JavaLens operational UX.

## Scope

### In Scope For This Repo

- project registration for Java projects
- starting, stopping, and restarting `javalens-mcp` instances
- per-project workspace/runtime management
- health and status visibility
- logs and recovery information
- generated MCP client configuration where useful
- polished desktop UX for multi-project operations

### Explicitly Out Of Scope

- modifying or forking upstream `javalens-mcp`
- reimplementing Java semantic analysis or refactoring
- unrelated editor plugin development

## Core Decisions

- Upstream `javalens-mcp` remains untouched.
- `javalens-manager` is a manager/orchestrator, not a semantic engine.
- One managed `javalens` instance is assumed per project/session.
- Multi-project support comes from managing multiple isolated `javalens` runtimes.
- The project should stay permissively licensed and compatible with upstream `javalens-mcp`.

## Current Product Shape

Build a stable desktop application that can:

- register projects
- launch and stop `javalens` instances
- surface runtime status and logs
- manage per-project workspaces and configuration
- help MCP clients connect cleanly

## Recommended Agent Team

Keep the team small and role-based:

- `orchestrator`: owns backlog slicing, coordination, and integration
- `requirements-analyst`: turns requirements into stories and acceptance criteria
- `platform-architect`: owns boundaries, SOLID design, and repo decisions
- `tauri-engineer`: owns manager implementation details
- `qa-test-engineer`: owns test strategy, regression coverage, and release confidence

## Immediate Planning Outputs

The next durable planning artifacts for this repo should be:

- ADR for repo purpose and boundaries
- ADR for runtime/process model
- ADR for configuration and workspace layout
- initial implementation backlog for Sprint 0 and Sprint 1
