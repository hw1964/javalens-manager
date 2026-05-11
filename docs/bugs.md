# javalens-manager — Bug Tracker

Living log of issues found during real-world usage. Mirrors the fork's [`docs/bugs.md`](https://github.com/hw1964/javalens-mcp/blob/master/docs/bugs.md) format so the two repos read the same.

Append new bugs at the **top**. Status values: `OPEN`, `IN_PROGRESS`, `FIXED in vX.Y.Z`, `WONTFIX`, `DUPLICATE`.

For each entry include: ID, date observed, severity, reproducer, expected vs actual, environment, and (when known) suspected root cause.

---

## #2 — Process-death flips workspace to `Stopped` instead of `Failed` (tray glyph stays gray, not red)

- **Status:** OPEN — known since Sprint 12; cut-lined into Sprint 13 (see `v0.13.0` plan's Verification §5: *"... within ~5s the popover's dot for that workspace flips to gray (or red, once Process-death → Failed lands; tracked separately in upgrade-checklist)"*); now formally filed.
- **Date observed:** Sprint 12 (~2026-04-23); re-flagged 2026-05-11.
- **Reporter:** Harald, via the agent-feedback / Sprint-13 cut-line trail.
- **Server version:** manager v0.13.1 (all v0.12.x and v0.13.x affected).
- **Severity:** LOW-MEDIUM — no data loss, no functional regression, but users can't tell at a glance whether a workspace was *intentionally stopped* or *externally killed*. Defeats half the tray-status promise from Sprint 12.

### Reproducer

1. Manager running with at least one workspace at status `running` (tray glyph `●`).
2. From a shell, `kill -9 <PID>` of the javalens.jar process for that workspace.
3. Wait 5 seconds for the 1s-poll cache-keyed change-detection to notice.
4. Observe the tray glyph and the dashboard runtime status.

### Expected

Per the Sprint 12 design and the v0.13.0 popover-plan Verification, the workspace transitions to `RuntimePhase::Failed`. Tray glyph flips to `✗` (red on the popover, glyph `✗` on the native menu). Dashboard shows the workspace card as "Failed" so the user can decide to restart or investigate.

### Actual

The polling sees the PID has exited and transitions the workspace to `RuntimePhase::Stopped`. Tray glyph flips to `○` (gray). Indistinguishable from a user-initiated *Stop*.

### Suspected root cause

The supervisor in `runtime_manager.rs` (or wherever the per-workspace poll lives) doesn't track *how* a workspace transitioned out of `running`. The state machine has `Starting → Running → Stopping → Stopped`, with no branch for unexpected exits. When polling notices the process is gone, it falls through to `Stopped` because that's the only terminal state defined.

### Suggested fix

1. Track an *expected-stopping* flag set by `commands::stop_runtime` and `manager_service::toggle_workspace` when the user (or peer-stop-all) initiates a clean shutdown.
2. On poll-detected exit:
   - If the expected-stopping flag was set within the last N seconds → transition to `Stopped` (current behaviour).
   - Otherwise → transition to `Failed` with the exit code (or "no exit code captured" if the child wait() lost the race with the kill).
3. Phase machine: extend with `Failed { exit_code: Option<i32>, killed_at: SystemTime }`. The 1s-poll cache-keyed comparator already treats phase changes as the trigger, so the tray glyph flips correctly once `Failed` is plumbed in.
4. UI side: `phase_glyph` in `src-tauri/src/lib.rs` already maps `Failed → "✗"`. The popover CSS already has `.dot--failed { background: #EF4444 }`. No frontend work needed.

### Why this got cut from Sprint 13

Sprint 13 was scoped to the tray-menu refinement against AppIndicator/GNOME constraints. Process-death detection is a state-machine concern, not a tray-rendering concern. The plan's Verification §5 said the cut was acceptable because users can still see the workspace name went non-green; calling it `Failed` vs `Stopped` is information-rich UX, not blocking-functional.

### Cross-reference

- Sprint 12 backlog originally promised this; deferred to v0.12.x patch and never landed.
- Sprint 13 popover plan acknowledged it as a separate concern.
- Fork's `docs/upgrade-checklist.md` Sprint 14 (v1.8.x) backlog section now references it (commit `91cbf5c` in the fork).

---

## #1 — Tauri webview renders blank on aarch64 / NVIDIA Grace (and some x86_64 GPU stacks)

- **Status:** PARTIAL FIX in v0.13.1 install.sh wrapper (commit `7f1f7b7`, 2026-05-11); **full fix pending v0.13.2** (bake the env-var into the `.deb` postinst and AppImage entry point so users who skip `install.sh` also get it).
- **Date observed:** 2026-05-11
- **Reporter:** Harald, on Gigabyte AI Top Atom (NVIDIA DGX Spark / GB10).
- **Server version:** manager v0.13.1 (`.deb` and `.AppImage` aarch64 builds from the v0.13.1 ARM CI matrix).
- **Severity:** HIGH on affected hosts — app is unusable (no UI). Affects 100% of aarch64 + NVIDIA Grace launches and an unknown fraction of x86_64 + NVIDIA-proprietary setups. Unaffected hosts (most Intel/AMD desktops) see no symptom.

### Environment

- Host: Gigabyte AI Top Atom / NVIDIA Project DIGITS / DGX Spark ("GB10").
- CPU: NVIDIA Grace (ARM Neoverse V2, aarch64).
- OS: NVIDIA's Ubuntu 24.04 fork for Spark DGX.
- Stack: Tauri 2.10.x + WebKitGTK 4.1 (≥ 2.42).

### Reproducer

1. `curl -sSL https://raw.githubusercontent.com/hw1964/javalens-manager/main/install.sh | bash` (or `sudo dpkg -i javalens-manager_0.13.1_arm64.deb`).
2. Launch `javalens-manager` from the app menu (or `~/.local/bin/javalens-manager`).

### Expected

Manager window opens showing Workspaces / Dashboard / Settings tabs and the registered projects.

### Actual

GTK window opens with the title bar and window controls (minimize / maximize / close). **The webview content area is a solid flat colour (GTK theme background).** Svelte UI never paints. No error on stderr beyond generic WebKit init log lines. The app doesn't crash — the JS just never gets composited.

Screenshot: blank dark-grey rectangle with only the `javalens-manager` title bar visible.

### Suspected root cause

WebKitGTK ≥ 2.42 added a DMABUF-based GPU compositor path. On NVIDIA Grace + Blackwell (and some x86_64 NVIDIA-proprietary stacks) the DMABUF init silently fails, the WebKit-managed surface is never composited, and we get a blank webview. Window chrome is fine because that's GTK's job. Same root cause as the upstream Tauri issue cluster (e.g. tauri-apps/tauri#9304 family).

### Fix

Set `WEBKIT_DISABLE_DMABUF_RENDERER=1` in the environment before launching the binary. Disables the DMABUF compositor and falls back to the previous WebKitGTK rendering path. Harmless on systems that don't need it.

**Shipped in install.sh as of commit `7f1f7b7`** — the script now writes a wrapper at `~/.local/bin/javalens-manager` that exports the env-var and `exec`s the real AppImage at `~/.local/bin/javalens-manager.AppImage`. Both shell launches and GNOME app-menu launches go through the wrapper. Existing users re-running `curl ... | bash` pick up the wrapper automatically.

### What's still missing (v0.13.2 follow-up)

`.deb`-installed users (without `install.sh`) still see the blank webview. Two ways to fix:

1. **`.deb` postinst hook** that writes the same wrapper into `/usr/bin/` (or wherever the .deb installs the launcher), pointing at the AppImage location. Or modifies the bundled `.desktop` to `Exec=env WEBKIT_DISABLE_DMABUF_RENDERER=1 …`.
2. **AppImage entry-point** — patch the AppImage's `AppRun` to `export WEBKIT_DISABLE_DMABUF_RENDERER=1` before invoking the bundled binary. That fixes both `.deb` and AppImage paths uniformly.

Option 2 is preferable (single fix, no per-package divergence). Tauri-bundler doesn't directly expose AppRun customisation, so the v0.13.2 implementation probably has to post-process the bundled AppImage during CI to inject the env-var.

### Cross-reference

- Memory: [`webkit_dmabuf_blank_webview.md`](/home/harald/.claude/projects/-home-harald-CursorProjects-javalens-manager/memory/webkit_dmabuf_blank_webview.md) for the diagnostic-ladder pattern reusable across any Tauri-on-Linux app.
- Same env-var has been recommended by Tauri's maintainers for several years; we're late to bake it in.
