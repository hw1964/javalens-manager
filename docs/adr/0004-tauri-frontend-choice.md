# ADR-0004: Initial Tauri Frontend Choice

- Status: Accepted
- Date: 2026-04-19

## Context

The current docs commit the project to a Tauri desktop application with a Rust backend, but they do not yet lock the first frontend implementation choice inside that shell.

Sprint 0 and Sprint 1 need a concrete bootstrap direction so the repository can move from planning into a first vertical slice without reopening the frontend question during scaffolding.

The broader requirements allow either `Svelte` or `Leptos`, but the immediate need is a fast path to:

- a project list
- runtime start/stop controls
- status and health display
- preferences and logs later

## Decision

The first `javalens-manager` bootstrap will use:

- Tauri for the desktop shell
- Rust for runtime/process orchestration
- Svelte for the frontend UI

This is the default for the first scaffold and early vertical slices. It does not prevent revisiting the UI stack later if the product requirements materially change.

## Rationale

- Svelte has a fast bootstrap path for Tauri and a small amount of UI boilerplate
- the early UI needs are form- and state-driven rather than framework-experimental
- Rust remains the clear home for process control, filesystem/state ownership, and platform integration
- this keeps the architecture aligned with the external requirements doc while minimizing setup friction

## Consequences

Positive:

- Sprint 1 can start from a concrete scaffold target
- frontend and backend responsibilities remain clear
- the team can focus on runtime behavior and UX instead of frontend framework comparison

Negative:

- a later switch to another frontend technology would have migration cost
- some contributors may prefer a Rust-only UI stack for conceptual consistency

## Revisit Criteria

Reconsider this ADR only if one of the following becomes true:

- Tauri plus Svelte blocks a required desktop capability
- the team decides to standardize on a Rust-only UI stack across multiple repos
- build, packaging, or maintainability costs prove materially worse than expected
