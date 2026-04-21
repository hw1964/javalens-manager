# javalens-manager Help

**javalens-manager** is a desktop companion for **JavaLens**: it registers Java projects on your machine, assigns each one a local port, starts and stops the JavaLens MCP runtime per project, and **deploys** MCP connection details into your AI tools (Cursor, Claude Desktop, Antigravity, IntelliJ-style configs).

Use **Dashboard** for day-to-day work, **Settings** for runtime paths and agent config files, and **Help** (this page) for orientation. Nothing here replaces your tools’ own documentation.

## Installation & Updates

To install or update **javalens-manager** on Linux, you can use the provided installation script. This script automatically downloads the latest `.AppImage` from GitHub Releases, verifies its checksum, and sets up a desktop entry so the app appears in your system launcher.

Run the following command in your terminal:

```bash
curl -sSL https://raw.githubusercontent.com/hw1964/javalens-manager/main/install.sh | bash
```

For more details or to download `.deb` packages manually, visit the [GitHub Releases page](https://github.com/hw1964/javalens-manager/releases).

---

## Dashboard

The Dashboard is split into three areas: a **left column** for adding projects and workspace import, a **right column** for the managed project list and **Agent deploy**, and a **full-width strip** at the bottom for the **currently selected** project’s status.

![Dashboard overview](/help/dashboard.png)

*Main Dashboard: project form and workspace import (left), managed projects and deploy toolbar (right), selected project status (bottom). Layout may stack on narrower windows.*

### Register Project

1. **Name** — Required. When you **Browse** for a folder, the name is filled from that folder’s last path segment (you can edit it). The placeholder reminds you it defaults from the folder.
2. **Project path** — The root directory of a Java/Maven/Gradle (or Eclipse PDE) project. Use **Browse** to pick a folder.
3. **Assigned port** — Each JavaLens instance for this project listens on this port. The manager suggests the next free port in your **permitted range** (see Settings) and checks for conflicts.
4. **Save project** — Registers the project; it then appears in **Managed Projects**.

### Import VSCode Workspace

Choose a `.code-workspace` file (**Browse**), then **Discover** to list Maven/Gradle and Eclipse/PDE Java projects. Tick the rows you want and click **Import selected** to register them in bulk. Use this when one workspace wraps many repos.

### Managed Projects

The list shows every registered project: path, assigned port (with **Set** to change it), and whether JavaLens is **RUNNING** or stopped. **Start**, **Stop**, and **Delete** apply to one row. At the top, **Start all**, **Stop all**, and **Delete all** act on every project—use **Delete** carefully; it removes the registration from the manager.

The summary line (totals and “all running” style summary) gives you a quick health read across projects.

### Agent deploy

The **Agent deploy** strip contains **Deploy to Agents**, **Dry run**, **Regenerate**, and **Delete**. These actions do **not** start or stop JavaLens; they build MCP entries from your **registered** projects and read or write **MCP client config files** on disk (see Settings → MCP Config Locations).

- **Deploy to Agents** — Writes manager-owned MCP server entries for **each registered project** the manager can resolve (JAR/workspace/command line), into the selected clients’ configs, plus related rule blocks the manager maintains. If the runtime or workspace for a project cannot be resolved, that project may be omitted and deploy may report validation issues—start runtimes and **Save settings** as needed so paths stay consistent.
- **Dry run** — Same validation and diff-style output as deploy, but **no files are written**. Use this to preview changes when unsure.
- **Regenerate** — Rewrites the manager-managed sections in the client configs even if the manager thinks nothing changed. Use after manual edits outside the app, or to recover from a half-written file.
- **Delete** — Removes **only** the manager-injected MCP servers and rule blocks from the selected clients. It does not uninstall JavaLens or remove your projects from the manager.

Clicking any of these opens a **target picker**: check **Cursor**, **Claude**, **Antigravity**, and/or **IntelliJ** for that run only. Default checkboxes for new runs come from each client’s **Deploy** toggle under **Settings → MCP Config Locations**.

### Selected Project Status

When you select a row in **Managed Projects**, the bottom panel shows **Name**, **Project path**, **Assigned port**, process id (**PID**) if running, and **Phase / Health** text from the runtime. Use **Refresh** on that panel if you want to re-query status without switching views.

---

## Settings

**Settings** uses a **two-by-two grid** (two columns, two rows) of cards: **JavaLens Runtime** and **Exposed Services** on the first row, **Machine Runtime Controls** and **MCP Config Locations** on the second. On small screens the grid may stack into a single column—there is **no** three-column layout by design.

The page is taller than a typical window; use the app’s scroll area to reach **Save settings** at the bottom. The screenshots below are **intentional partial captures** of the top and bottom of the same page.

![Settings — JavaLens Runtime and Exposed Services](/help/settings-top.png)

*Upper part of Settings (scroll down for the rest): **JavaLens Runtime** (left) and **Exposed Services** (right).*

![Settings — Machine controls and MCP locations](/help/settings-bottom.png)

*Lower part of Settings: **Machine Runtime Controls** (left) and **MCP Config Locations** (right), including merge options and the global **Save settings** bar.*

### JavaLens Runtime

Controls how the **global** JavaLens binary is sourced and updated:

- **Global JavaLens Source** — **Managed runtime** uses the copy the manager downloads and tracks; **Local JAR fallback** lets you point at a specific `javalens` JAR on disk (with **Browse**).
- **Active** — Shows the version of the managed runtime when applicable.
- **Update policy** — **Ask before updating** vs **Always keep latest** (behavior aligns with your confirmation preferences elsewhere).
- **Check upstream JavaLens release on dashboard load** — When enabled, the manager checks for newer releases when you open the Dashboard.
- **Download latest** / **Download update** — Fetches or updates the managed runtime (wording depends on whether an update is available).
- **Refresh release info** — Re-queries release metadata without downloading.

Status chips (**Status**, **Latest**, **Checked**) summarize the last release check. **Checked** may show a machine-readable timestamp from the updater—treat it as “when we last asked upstream,” not a clock for humans.

### Exposed Services

**Test Services** runs a **live MCP handshake** against JavaLens and lists **tool** names and descriptions the server exposes (count and duration appear after a successful probe). Use this to confirm the runtime is up and the MCP surface matches expectations—especially after version changes.

If a probe fails, read the error text in the panel; fix connectivity or runtime issues before relying on **Deploy to Agents**.

### Machine Runtime Controls

- **Manager data root** — Base directory for caches, logs, and other machine-local data ( **Browse** to change). **Use system tray** toggles whether the app integrates with the system tray where supported.
- **Permitted project ports** — Inclusive range of TCP ports the manager may assign to projects. It assigns **one port per project** and avoids clashes with other assignments.
- **Diagnostics** — Read-only paths for the **projects** store, **settings** file, **state** directory, and resolved **data root** (useful when reporting issues or backing up).

Under diagnostics, **Clean logs** removes manager runtime logs (projects and settings stay). **Clean workspaces** removes workspace/index caches. **Start from scratch** runs both cleanups; stop runtimes first if you use it.

### MCP Config Locations

For each supported client (**Cursor**, **Claude**, **Antigravity**, **IntelliJ**):

- **Deploy** — When checked, this client is included in the **default** set when you open the deploy target picker. You can still override per run.
- **Current** — Effective path the manager will use (auto-detected path or your **Manual override path**).
- **Manual override path** — Use if the config file lives somewhere non-standard; **Browse** / **Clear** assist editing.

**Redetect defaults** re-runs auto-detection for standard install locations.

**Merge mode** controls how written client configs combine with existing content:

- **Safe merge** — Inserts or updates only the manager-owned blocks, preserving unrelated entries where possible.
- **Replace managed section** — Replaces the entire manager-delimited section (stronger reset; still scoped to what the manager owns).

**Create backup before MCP config write** — When enabled, the manager writes a timestamped backup next to the config before changing it. Recommended when experimenting with **Merge mode** or new paths.

---

## Quick reference

| Goal | Where to go |
|------|------------------|
| Register or import projects | Dashboard (left) |
| Start/stop JavaLens for a project | Managed Projects (per row or bulk) |
| Push MCP URLs into Cursor / Claude / etc. | Dashboard → **Agent deploy** |
| Change ports or data directory | Settings → **Machine Runtime Controls** |
| Point deploy at custom MCP config paths | Settings → **MCP Config Locations** |
| Verify JavaLens exposes MCP tools | Settings → **Exposed Services** → **Test Services** |

If something fails, check the **Diagnostics** paths for logs and settings files, run **Dry run** before **Deploy**, and keep **Create backup before MCP config write** on until you trust your layout.
