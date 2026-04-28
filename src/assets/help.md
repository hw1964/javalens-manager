# javalens-manager Help

**javalens-manager** is the desktop control plane for **JavaLens** — it lets you create **named workspaces** of one-or-more Java projects, runs a single shared JavaLens MCP service per workspace, and **deploys** the connection details into your AI tools (Cursor, Claude Desktop, Antigravity, IntelliJ-style configs).

The point: it gives your AI agents the same IDE-grade understanding of a Java codebase that a human developer gets in Eclipse or IntelliJ — call hierarchies, type hierarchies, references, refactorings, build classpath, JDK semantics. **Java agents on steroids.**

Use **Dashboard** for day-to-day work, **Settings** for runtime paths and agent config files, and **Help** (this page) for orientation.

## Installation & Updates

To install or update **javalens-manager** on Linux, you can use the provided installation script. It downloads the latest `.AppImage` from GitHub Releases, verifies its checksum, and registers a desktop entry:

```bash
curl -sSL https://raw.githubusercontent.com/hw1964/javalens-manager/main/install.sh | bash
```

For `.deb` packages or other formats, see the [GitHub Releases page](https://github.com/hw1964/javalens-manager/releases).

---

## Workspaces — the core concept

A **workspace** is a named group of Java projects loaded into one JavaLens process and exposed to agents as **one MCP service** (`jl-<workspace-name>`). The agent sees the combined symbol set of every project in the workspace; cross-project navigation, find-references, and (in fork v1.4.0+) refactorings work across the whole group.

- **One workspace per cohesive task.** A bundle/multi-module application (e.g. JATS with 12 OSGi bundles), a monorepo, or a single project that you want isolated — each gets its own workspace.
- **Live updates.** Add or remove a project from a workspace and the running JavaLens picks it up within ~1 second through a `workspace.json` file watcher. No MCP-client restart, no agent-session reload.
- **No ports.** Workspaces are identified by name. There is no port range, no per-project port allocation, no port conflicts.
- **Tool budget.** Each workspace contributes ~66 tools toward the agent's tool registration cap (Antigravity caps around 100). Stick to 1–3 active workspaces concurrently.
- **Migration.** If you're upgrading from v0.10.3 or earlier, existing projects are auto-grouped into default workspaces named like `workspace-11100` (derived from the old `assignedPort`). Rename them through the Workspaces card or the workspace header.

---

## Dashboard

![Dashboard overview](/help/dashboard.png)

*Workspaces card and Register Project on the left; grouped Managed Projects with the Agent deploy strip on the right; selected project status across the bottom.*

The Dashboard splits into three areas:
- **Left column** — the **Workspaces** card (pick / create / rename / delete) and the **Register Project / Import VSCode Workspace** forms below it.
- **Right column** — the **Managed Projects** view grouped by workspace, the **Agent deploy** toolbar, and the bulk-action bar that appears when you select projects.
- **Bottom strip** — full-width **Selected Project Status** for the row you most recently picked.

### Workspaces card (left)

Each row in the Workspaces card shows a workspace name, a colored **status lamp**, and the project count.

- **Status lamp colors** — slate (stopped), amber (starting / mixed), emerald (running), coral (failed). The color reflects the workspace's aggregate runtime phase, derived from its members.
- **Click** a row to make that workspace the **active** one — newly registered projects join it, and the Register Project / Import forms update their hint accordingly.
- **+ New workspace…** — inline-creates an empty workspace. It pins until either you add a project to it or you delete it.
- **Hover** a row to reveal the rename ✎ and delete ✕ icons. **Right-click** for a context menu with Rename / Delete.

### Register Project

1. **Name** — Required. Browsing for a folder fills this in from the folder's last segment (you can edit it).
2. **Project path** — The root directory of a Java/Maven/Gradle (or Eclipse PDE) project.
3. **Workspace** — Implicitly the active workspace from the left card. Pick a different one in the Workspaces card to switch.
4. **Save project** — Registers the project. The manager updates the workspace's `workspace.json` and any running JavaLens picks up the new project immediately.

### Import VSCode Workspace

Pick a `.code-workspace` file (**Browse**), then **Discover** to enumerate Maven/Gradle and Eclipse/PDE Java projects. Tick the rows you want and click **Import selected** — every imported project joins the currently active workspace.

### Managed Projects (grouped view)

The right pane shows one **workspace card** per workspace, with project rows nested inside. Each card has a header with the workspace name, status badge, project count, and per-workspace **Start workspace / Stop workspace / Delete workspace** actions. Click the chevron to collapse or expand the card.

Each project row inside a workspace card has:
- A **selection checkbox** on the left (see "Bulk actions" below).
- The **project name** (click to make it the *Selected project* shown in the bottom strip; click again to inline-rename).
- The **project path** below the name.
- **Refresh / Status badge / Start / Stop / Delete** on the right.
- **Right-click** for a context menu: Start project / Stop project / Rename project / Move to workspace… / Delete project.

At the very top of the pane, a metric strip shows totals: workspaces, running, stopped, projects.

### Bulk actions (multi-select)

Use the per-row checkboxes to build a **cross-workspace** selection set. Shift-click to extend a range; ctrl/cmd-click toggles a single row.

When at least one row is selected, a **bulk-action bar** appears above the workspace cards:
- **Move to workspace ▾** — move every selected project to a chosen (existing or new) workspace in one go.
- **Start selected** / **Stop selected** — fan the per-project start/stop out over the selection.
- **✕** — clear the selection.

### Drag-and-drop

Project rows are draggable. Grab any row and drop it on:
- A **workspace card header** in the right pane, or
- A **workspace row** in the left Workspaces list (handy when the destination card is collapsed or out of view).

If the row you grab is part of an active selection, the **whole selection** moves with it. Dragging an unselected row carries just that one row and leaves the selection intact. The source row dims and the drop target outlines while you drag; Esc cancels.

### Agent deploy

The **Agent deploy** strip contains **Deploy to Agents**, **Dry run**, **Regenerate**, and **Delete**. These actions do **not** start or stop JavaLens — they rebuild MCP entries from your workspaces and read or write **MCP client config files** on disk (see Settings → MCP Config Locations).

- **Deploy to Agents** — Writes manager-owned MCP server entries (one per workspace, keyed `jl-<workspace-name>`) into the selected clients' configs, plus the rule blocks the manager maintains.
- **Dry run** — Same validation and diff output as Deploy, but no files are written.
- **Regenerate** — Force-rewrites the manager-managed sections, even if nothing has changed since the last write. Useful after manual edits.
- **Delete** — Removes only the manager-injected MCP servers and rule blocks from the selected clients. It does not uninstall JavaLens or remove your projects.

Each of these opens a **target picker**: check Cursor / Claude / Antigravity / IntelliJ for that run only. Defaults come from each client's **Deploy** toggle under Settings → MCP Config Locations.

**Cursor (length limit):** Cursor rejects tools when `serverName + ":" + toolName` exceeds about **59–60** characters. The manager keeps the generated `jl-` ids short so the longest JavaLens tool names still fit. **Antigravity** instead caps the total *number* of MCP tools registered across all servers (around 100 in current builds) — that is a separate constraint, and the main reason to keep concurrent workspaces small.

### Tool surface (fork v1.5.0)

JavaLens v1.5.0 registers **55 tools per workspace service** (down from 66 in v1.4.0). Two parametric tools replaced 13 narrow ones so multi-workspace setups have headroom under Antigravity's 100-tool cap.

- **`find_pattern_usages(kind, query)`** — type-anchored searches. `kind ∈ { annotation, instantiation, type_argument, cast, instanceof }`. Replaces `find_annotation_usages` / `find_type_instantiations` / `find_type_arguments` / `find_casts` / `find_instanceof_checks`.
- **`find_quality_issue(kind, ...)`** — code-quality analyses. `kind ∈ { naming, bugs, unused, large_classes, circular_deps, reflection, throws, catches }`. Replaces `find_naming_violations` / `find_possible_bugs` / `find_unused_code` / `find_large_classes` / `find_circular_dependencies` / `find_reflection_usage` / `find_throws_declarations` / `find_catch_blocks`.

Each parametric tool's `kind` is a typed enum in the input schema with per-kind descriptions, so agents can discover what's available through `tools/list`.

`find_method_references` and the position-anchored search tools (`find_references`, `find_implementations`, `find_field_writes`, `find_tests`) stay as separate tools — their parameter shapes don't fit the consolidated kind-dispatched pattern.

Fork **v1.5.1** added five JDT-LTK structural refactoring tools (Sprint 11 Phase E): `move_class`, `move_package`, `pull_up`, `push_down`, `encapsulate_field`. They take a position (filePath / line / column zero-based) plus refactoring-specific arguments — see the per-tool descriptions exposed via `tools/list` for the exact parameters. Per-workspace tool count after v1.5.1: **60**.

Known limitation in v1.5.1: `move_class` / `pull_up` / `push_down` / `encapsulate_field` use JDT's import-rewrite path which expects Eclipse JDT.UI preference defaults to be registered. Headless RCP runs (without `org.eclipse.jdt.ui` on the target platform) may need the workspace's `.metadata` to retain those defaults from a previous Eclipse run. `move_package` doesn't depend on this and works unconditionally. The full preference-init plumbing lands in v1.5.2.

### Selected Project Status

When you click a project row, the bottom strip shows **Name**, **Project path**, **Workspace**, the **PID** of that workspace's JavaLens process (if running), and the **Phase / Health** detail from the runtime. Multiple projects in the same workspace share a PID. Use the refresh icon on that strip to re-query without switching views.

---

## Settings

![Settings — JavaLens Runtime and Exposed Services](/help/settings-top.png)

*Top half of the Settings page: JavaLens Runtime and Exposed Services.*

![Settings — Machine controls and MCP locations](/help/settings-bottom.png)

*Bottom half: Machine Runtime Controls (with Diagnostics workspace counts) and MCP Config Locations.*

Settings is a **two-by-two grid**: JavaLens Runtime and Exposed Services on the first row, Machine Runtime Controls and MCP Config Locations on the second. The page can be taller than the window — scroll to reach **Save settings** at the bottom.

### JavaLens Runtime

Controls how the global JavaLens binary is sourced and updated:

- **Release source** — `hw1964/javalens-mcp` (recommended fork) or upstream / custom. Switching saves and downloads the latest release from the new source.
- **Global JavaLens Source** — **Managed runtime** uses the binary the manager downloads and tracks; **Local JAR fallback** points at a specific `javalens.jar` on disk.
- **Active** — Version of the managed runtime, when applicable.
- **Update policy** — *Ask before updating* vs *Always keep latest*.
- **Auto-check release source on dashboard load** — When enabled, the manager checks for newer releases when you open the Dashboard.
- **Download update** — Appears when an update is available; fetches and installs it.

### Exposed Services

**Test Services** runs a live MCP handshake against JavaLens and lists the tool names and descriptions the server exposes (count and duration appear after a successful probe). Use this to confirm the runtime is reachable and that the tool surface matches expectations after a version change.

If a probe fails, fix connectivity or runtime issues before relying on **Deploy to Agents**.

### Machine Runtime Controls

- **Manager data root** — Base directory for caches, logs, and JDT workspace indexes. Each workspace's data lives under `<data_root>/workspaces/<workspace-name>/` (which is also where `workspace.json` is written).
- **Use system tray** — When enabled, closing the window keeps the manager running in the system tray.
- **Diagnostics** — Read-only summary: paths for the projects store, settings file, state directory, and resolved data root. **Workspaces** and **Project count** mirror the Dashboard totals, useful when reporting issues.
- **Clean logs** — Removes manager runtime logs (workspaces and settings stay).
- **Clean workspaces** — Removes JDT workspace caches (forces re-index next start).
- **Start from scratch** — Runs both cleanups; stop runtimes first.

### MCP Config Locations

For each supported client (**Cursor**, **Claude**, **Antigravity**, **IntelliJ**):

- **Deploy** — When checked, the client is included in the *default* set of the deploy target picker. Override per run if you need to.
- **Current** — Effective path the manager will use (auto-detected, or your manual override).
- **Manual override path** — Use when the config file lives somewhere non-standard.

**Redetect defaults** re-runs auto-detection. **Antigravity (Google / Gemini):** the manager looks in several common locations including `~/.gemini/antigravity/mcp_config.json`. Antigravity caps total registered MCP tools (≈100), so keep concurrent workspaces small.

**Merge mode**:
- **Safe merge** — inserts or updates only the manager-owned blocks, preserving unrelated entries.
- **Replace managed section** — replaces the entire manager-delimited section. Stronger reset, still scoped to what the manager owns.

**Create backup before MCP config write** writes a timestamped backup next to each config before changes. Recommended while you're experimenting.

---

## Quick reference

| Goal | Where to go |
|------|------------------|
| Create or rename a workspace | Dashboard → Workspaces card (left) |
| Register or import projects | Dashboard left column |
| Move a project to another workspace | Right-click row → *Move to workspace…* OR drag the row onto a workspace |
| Bulk-move projects | Tick checkboxes → *Move to workspace ▾* |
| Start/stop a workspace's JavaLens | Workspace header in Managed Projects |
| Push MCP entries into Cursor / Claude / etc. | Dashboard → **Agent deploy** |
| Change data root or system-tray behavior | Settings → **Machine Runtime Controls** |
| Point deploy at custom MCP config paths | Settings → **MCP Config Locations** |
| Verify JavaLens exposes MCP tools | Settings → **Exposed Services** → **Test Services** |
| Find logs / settings files for a bug report | Settings → **Diagnostics** |

If something fails: check Diagnostics for paths, run **Dry run** before **Deploy**, and keep **Create backup before MCP config write** on until you trust your layout.
