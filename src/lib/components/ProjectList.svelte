<script lang="ts">
  import { afterUpdate } from "svelte";
  import type {
    DeployTargetFlags,
    DeployMode,
    DeployToAgentsResult,
    ProjectRecord,
    RuntimeStatusRecord
  } from "../api/tauri";
  import ContextMenu from "./ContextMenu.svelte";

  interface ContextMenuItem {
    label: string;
    onSelect: () => void;
    danger?: boolean;
    disabled?: boolean;
  }

  export let projects: ProjectRecord[] = [];
  export let runtimeStatuses: Record<string, RuntimeStatusRecord> = {};
  export let projectErrors: Record<string, string> = {};
  export let selectedProjectId: string | undefined;
  export let disabled = false;

  export let onSelect: (projectId: string) => void;
  export let onStart: (projectId: string) => void;
  export let onStop: (projectId: string) => void;
  export let onDelete: (projectId: string) => void;
  export let onRefresh: (projectId: string) => void;
  export let onStartAll: () => void;
  export let onStopAll: () => void;
  export let onDeleteAll: () => void;
  /** Sprint 10 v0.10.4: move a project to a different (existing or new) workspace. */
  export let onSetWorkspace: (projectId: string, workspaceName: string) => void;
  export let onRenameWorkspace: (oldName: string, newName: string) => void;
  export let onDeleteWorkspace: (workspaceName: string) => void;
  /** All workspaces the UI knows about — for the Move dropdown and
   * for rendering empty workspace cards (newly-created without
   * projects). Owner is App.svelte. */
  export let knownWorkspaces: string[] = [];
  export let onDeploy: (mode: DeployMode, targetClients?: string[]) => void;
  export let deployTargetDefaults: DeployTargetFlags = {
    cursor: true,
    claude: true,
    antigravity: true,
    intellij: true
  };
  export let deployBusy = false;
  export let deployError: string | undefined;
  export let lastDeployResult: DeployToAgentsResult | undefined;

  const deployTargetOptions: Array<{ key: keyof DeployTargetFlags; label: string }> = [
    { key: "cursor", label: "Cursor" },
    { key: "claude", label: "Claude" },
    { key: "antigravity", label: "Antigravity" },
    { key: "intellij", label: "IntelliJ" }
  ];

  let rowRefs: Record<string, HTMLElement> = {};
  let lastAutoScrolledSelection: string | undefined;
  let showDeployTargetPicker = false;
  let pendingDeployMode: DeployMode = "deploy";
  let deployTargetsDraft: DeployTargetFlags = { ...deployTargetDefaults };

  /** Sprint 10 v0.10.4: workspace-grouping state. */
  /** Workspaces collapsed in the UI. Default: all expanded. */
  let collapsedWorkspaces: Record<string, boolean> = {};
  /** Which workspace is being renamed inline (null = none). */
  let renamingWorkspace: string | null = null;
  let renameDraft = "";
  let renameError = "";
  /** Which project's "Move…" dropdown is open (null = none). One row
   * at a time; clicking Move on another row replaces the open one. */
  let movingProjectId: string | null = null;
  /** Selection in the Move dropdown. Special value `"__new__"` means
   * the user is typing a new workspace name in moveNewName. */
  let moveSelection = "";
  let moveNewName = "";

  /** Currently-open right-click context menu. Closed = null. */
  let contextMenu: { x: number; y: number; items: ContextMenuItem[] } | null = null;

  function openProjectContextMenu(event: MouseEvent, project: ProjectRecord) {
    event.preventDefault();
    if (disabled) return;
    contextMenu = {
      x: event.clientX,
      y: event.clientY,
      items: [
        { label: "Start", onSelect: () => onStart(project.id) },
        { label: "Stop", onSelect: () => onStop(project.id) },
        { label: "Move to workspace…", onSelect: () => openMoveDropdown(project) },
        {
          label: "Delete project",
          danger: true,
          onSelect: () => onDelete(project.id),
        },
      ],
    };
  }

  function openWorkspaceContextMenu(
    event: MouseEvent,
    workspace: { name: string; projects: ProjectRecord[] },
  ) {
    event.preventDefault();
    if (disabled) return;
    contextMenu = {
      x: event.clientX,
      y: event.clientY,
      items: [
        { label: "Start workspace", onSelect: () => startWorkspace(workspace.projects) },
        { label: "Stop workspace", onSelect: () => stopWorkspace(workspace.projects) },
        { label: "Rename workspace", onSelect: () => startRenameWorkspace(workspace.name) },
        {
          label: "Delete workspace",
          danger: true,
          onSelect: () => handleDeleteWorkspace(workspace.name, workspace.projects.length),
        },
      ],
    };
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  function registerRow(node: HTMLElement, projectId: string) {
    rowRefs[projectId] = node;
    return {
      destroy() {
        delete rowRefs[projectId];
      }
    };
  }

  function handleDeleteAll() {
    if (confirm("Delete all projects and stop all runtimes?")) {
      onDeleteAll();
    }
  }

  function deploySummary(result: DeployToAgentsResult): string {
    const succeeded = result.clients.filter((entry) => entry.status === "success").length;
    const total = result.clients.length;
    const failed = result.clients.filter((entry) => entry.status === "failed").length;
    const skipped = result.clients.filter((entry) => entry.status === "skipped").length;
    const actionLabel = result.mode === "delete" ? "updated" : "deployed";
    return `${result.mode}: ${actionLabel} ${succeeded}/${total} clients (${succeeded} success, ${failed} failed, ${skipped} skipped)`;
  }

  function deployFailureDetails(result: DeployToAgentsResult): string[] {
    return result.clients
      .filter((entry) => entry.status === "failed")
      .flatMap((entry) => {
        const validationErrors = entry.validationErrors?.length ? entry.validationErrors : [entry.message];
        return validationErrors.map((detail) => `${entry.client}: ${detail}`);
      });
  }

  function deploySkippedDetails(result: DeployToAgentsResult): string[] {
    return result.clients
      .filter((entry) => entry.status === "skipped")
      .map((entry) => `${entry.client}: ${entry.message}`);
  }

  function openDeployTargetPicker(mode: DeployMode) {
    pendingDeployMode = mode;
    deployTargetsDraft = { ...deployTargetDefaults };
    showDeployTargetPicker = true;
  }

  function closeDeployTargetPicker() {
    showDeployTargetPicker = false;
  }

  function toggleDeployTarget(client: keyof DeployTargetFlags) {
    deployTargetsDraft = {
      ...deployTargetsDraft,
      [client]: !deployTargetsDraft[client]
    };
  }

  function runDeployWithTargets() {
    const selectedTargets = deployTargetOptions
      .filter((option) => deployTargetsDraft[option.key])
      .map((option) => option.key);
    onDeploy(pendingDeployMode, selectedTargets);
    closeDeployTargetPicker();
  }

  function runActionLabel(mode: DeployMode): string {
    if (mode === "delete") {
      return "Delete selected";
    }
    return `Run ${mode}`;
  }

  function extractProjectError(status?: RuntimeStatusRecord): string | null {
    if (!status) {
      return null;
    }

    const detail = status.detail?.trim();
    if (!detail) {
      return status.phase === "failed" ? "Runtime failed to start." : null;
    }

    const lowered = detail.toLowerCase();
    if (
      status.phase === "failed" ||
      lowered.includes("address already in use") ||
      lowered.includes("port already in use")
    ) {
      return detail;
    }

    return null;
  }

  /** Sprint 10 v0.10.4: open the inline Move dropdown for a project.
   * Replaces the old window.prompt() — we already know the workspace
   * names from `knownWorkspaces` so the user picks rather than types. */
  function openMoveDropdown(project: ProjectRecord) {
    movingProjectId = project.id;
    // Default to the first workspace that isn't this project's current.
    const others = knownWorkspaces.filter((n) => n !== project.workspaceName);
    moveSelection = others[0] ?? "__new__";
    moveNewName = "";
  }

  function cancelMove() {
    movingProjectId = null;
    moveSelection = "";
    moveNewName = "";
  }

  function commitMove(project: ProjectRecord) {
    let target =
      moveSelection === "__new__" ? moveNewName.trim() : moveSelection;
    if (!target || target === project.workspaceName) {
      cancelMove();
      return;
    }
    onSetWorkspace(project.id, target);
    cancelMove();
  }

  function toggleWorkspaceCollapsed(name: string) {
    collapsedWorkspaces = {
      ...collapsedWorkspaces,
      [name]: !collapsedWorkspaces[name]
    };
  }

  function startRenameWorkspace(name: string) {
    renamingWorkspace = name;
    renameDraft = name;
    renameError = "";
  }

  function cancelRenameWorkspace() {
    renamingWorkspace = null;
    renameDraft = "";
    renameError = "";
  }

  function commitRenameWorkspace() {
    if (renamingWorkspace === null) return;
    const trimmed = renameDraft.trim();
    if (trimmed.length === 0) {
      renameError = "Workspace name cannot be empty.";
      return;
    }
    if (trimmed === renamingWorkspace) {
      cancelRenameWorkspace();
      return;
    }
    onRenameWorkspace(renamingWorkspace, trimmed);
    cancelRenameWorkspace();
  }

  function handleRenameKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      commitRenameWorkspace();
    } else if (event.key === "Escape") {
      event.preventDefault();
      cancelRenameWorkspace();
    }
  }

  function handleDeleteWorkspace(name: string, projectCount: number) {
    const detail =
      projectCount === 0
        ? `Delete workspace "${name}"?`
        : `Delete workspace "${name}" and all ${projectCount} project(s) inside it?\n\nThis stops the workspace's javalens process and removes the JDT data dir on disk. Project paths on your filesystem are not touched.`;
    if (window.confirm(detail)) {
      onDeleteWorkspace(name);
    }
  }

  function startWorkspace(workspaceProjects: ProjectRecord[]) {
    // First member triggers the spawn; subsequent members join the
    // running process (runtime_manager dedupes via membership).
    for (const project of workspaceProjects) {
      onStart(project.id);
    }
  }

  function stopWorkspace(workspaceProjects: ProjectRecord[]) {
    // Each per-project stop_runtime call removes that project from the
    // workspace's member set. The last leaver kills the workspace process.
    for (const project of workspaceProjects) {
      onStop(project.id);
    }
  }

  /** Reduce a list of per-project phases to a single workspace phase.
   * Empty workspaces are stopped by definition; uniform-running members
   * → running; uniform-stopped → stopped; anything mixed → starting. */
  type Phase = "running" | "stopped" | "starting" | "failed";
  function deriveWorkspacePhase(phases: Phase[]): Phase {
    if (phases.length === 0) return "stopped";
    if (phases.every((p) => p === "running")) return "running";
    if (phases.every((p) => p === "stopped")) return "stopped";
    return "starting";
  }

  $: phases = projects.map((project) => runtimeStatuses[project.id]?.phase ?? "stopped");
  $: aggregatePhase =
    phases.length === 0
      ? "stopped"
      : phases.every((phase) => phase === "running")
        ? "running"
        : phases.every((phase) => phase === "stopped")
          ? "stopped"
          : "starting";
  $: aggregateLabel =
    aggregatePhase === "running"
      ? "all running"
      : aggregatePhase === "stopped"
        ? "all stopped"
        : "mixed";
  $: totalProjects = projects.length;
  $: runningProjects = projects.filter((project) => runtimeStatuses[project.id]?.phase === "running").length;
  $: stoppedProjects = totalProjects - runningProjects;

  /** Sprint 10 v0.10.4: group projects by workspace_name and compute
   * per-workspace aggregate status. Includes every workspace from
   * `knownWorkspaces` even when it has zero projects (so a newly-
   * created empty workspace is visible on both sides). Sorted
   * alphabetically. Defensively dedupes by project.id. */
  $: groupedWorkspaces = (() => {
    const seenIds = new Set<string>();
    const byName: Record<string, ProjectRecord[]> = {};
    // Seed every known workspace (so empties are rendered too).
    for (const name of knownWorkspaces) {
      byName[name] = [];
    }
    for (const project of projects) {
      if (seenIds.has(project.id)) continue;
      seenIds.add(project.id);
      const name = project.workspaceName || "workspace-default";
      if (!byName[name]) {
        byName[name] = [];
      }
      byName[name].push(project);
    }
    const order = Object.keys(byName).sort();
    return order.map((name) => {
      const ws_projects = byName[name];
      const ws_phases = ws_projects.map(
        (p) => (runtimeStatuses[p.id]?.phase ?? "stopped") as Phase
      );
      const ws_running = ws_projects.filter(
        (p) => runtimeStatuses[p.id]?.phase === "running"
      ).length;
      return {
        name,
        projects: ws_projects,
        phase: deriveWorkspacePhase(ws_phases),
        runningCount: ws_running,
      };
    });
  })();

  $: totalWorkspaces = groupedWorkspaces.length;
  $: runningWorkspaces = groupedWorkspaces.filter((w) => w.phase === "running").length;
  $: stoppedWorkspaces = groupedWorkspaces.filter((w) => w.phase === "stopped").length;
  $: selectedDeployTargetCount = deployTargetOptions.filter((option) => deployTargetsDraft[option.key]).length;
  $: highlightedDeployMode = showDeployTargetPicker ? pendingDeployMode : "deploy";

  afterUpdate(() => {
    if (!selectedProjectId || selectedProjectId === lastAutoScrolledSelection) {
      return;
    }
    const target = rowRefs[selectedProjectId];
    if (!target) {
      return;
    }
    target.scrollIntoView({ block: "nearest", behavior: "smooth" });
    lastAutoScrolledSelection = selectedProjectId;
  });
</script>

<section class="panel stack project-list-panel">
  <div class="project-list-header">
    <div>
      <h2>
        Managed Projects
        <span class={`aggregate-pill ${aggregatePhase}`}>
          <span class={`status-lamp ${aggregatePhase}`}></span>
          {aggregateLabel}
        </span>
      </h2>
      <p class="muted">Project services managed by the manager, grouped by workspace.</p>
      <div class="project-summary-metrics">
        <span class="metric-pill">Workspaces {totalWorkspaces}</span>
        <span class="metric-pill running">Running {runningWorkspaces}</span>
        <span class="metric-pill stopped">Stopped {stoppedWorkspaces}</span>
        <span class="metric-pill">Projects {totalProjects}</span>
      </div>
    </div>
    <div class="project-list-toolbar segmented-actions">
      <button disabled={disabled || projects.length === 0} on:click={() => onStartAll()} type="button">
        Start all
      </button>
      <button disabled={disabled || projects.length === 0} on:click={() => onStopAll()} type="button">
        Stop all
      </button>
      <button disabled={disabled || projects.length === 0} on:click={handleDeleteAll} type="button">
        Delete all
      </button>
    </div>
  </div>

  <div class="deploy-toolbar-wrap">
    <span class="deploy-toolbar-label">Agent deploy</span>
    <div class="deploy-toolbar segmented-actions">
    <button
      class:active={highlightedDeployMode === "deploy"}
      disabled={disabled || deployBusy}
      on:click={() => openDeployTargetPicker("deploy")}
      type="button"
    >
      {deployBusy ? "Deploying..." : "Deploy to Agents"}
    </button>
    <button
      class:active={highlightedDeployMode === "dryRun"}
      disabled={disabled || deployBusy}
      on:click={() => openDeployTargetPicker("dryRun")}
      type="button"
    >
      Dry run
    </button>
    <button
      class:active={highlightedDeployMode === "regenerate"}
      disabled={disabled || deployBusy}
      on:click={() => openDeployTargetPicker("regenerate")}
      type="button"
    >
      Regenerate
    </button>
    <button
      class:active={highlightedDeployMode === "delete"}
      disabled={disabled || deployBusy}
      on:click={() => openDeployTargetPicker("delete")}
      type="button"
    >
      Delete
    </button>
    </div>
  </div>

  {#if showDeployTargetPicker}
    <div class="deploy-target-picker">
      <p class="hint"><strong>Targets for {pendingDeployMode}</strong></p>
      <div class="deploy-target-grid">
        {#each deployTargetOptions as option}
          <label class="checkbox-row compact">
            <input
              checked={deployTargetsDraft[option.key]}
              disabled={disabled || deployBusy}
              on:change={() => toggleDeployTarget(option.key)}
              type="checkbox"
            />
            <span>{option.label}</span>
          </label>
        {/each}
      </div>
      <div class="actions compact">
        <button disabled={disabled || deployBusy} on:click={closeDeployTargetPicker} type="button">Cancel</button>
        <button
          disabled={disabled || deployBusy || selectedDeployTargetCount === 0}
          on:click={runDeployWithTargets}
          type="button"
        >
          {runActionLabel(pendingDeployMode)}
        </button>
      </div>
      {#if selectedDeployTargetCount === 0}
        <p class="project-error">Select at least one deploy target.</p>
      {/if}
    </div>
  {/if}

  {#if deployError}
    <p class="project-error">{deployError}</p>
  {:else if lastDeployResult}
    <p class="hint">{deploySummary(lastDeployResult)}</p>
    {#if deployFailureDetails(lastDeployResult).length > 0}
      {#each deployFailureDetails(lastDeployResult) as failure}
        <p class="project-error">{failure}</p>
      {/each}
    {/if}
    {#if deploySkippedDetails(lastDeployResult).length > 0}
      {#each deploySkippedDetails(lastDeployResult) as skipped}
        <p class="hint">{skipped}</p>
      {/each}
    {/if}
  {/if}

  {#if projects.length === 0}
    <div class="empty-state">
      No projects registered yet. Configure JavaLens first, then add a Java project on the left.
    </div>
  {:else}
    <div class="stack project-list-scroll">
      {#each groupedWorkspaces as workspace (workspace.name)}
        {@const collapsed = collapsedWorkspaces[workspace.name] ?? (workspace.phase === "stopped")}
        {@const isRenaming = renamingWorkspace === workspace.name}
        <article class="workspace-card" on:contextmenu={(e) => openWorkspaceContextMenu(e, workspace)}>
          <header class="workspace-header">
            <div class="workspace-title">
              <button
                aria-label={collapsed ? "Expand workspace" : "Collapse workspace"}
                class={`workspace-toggle ${collapsed ? "collapsed" : ""}`}
                on:click={() => toggleWorkspaceCollapsed(workspace.name)}
                title={collapsed ? "Expand" : "Collapse"}
                type="button"
              >
                <span class="workspace-toggle-icon">▾</span>
              </button>
              {#if isRenaming}
                <input
                  aria-label="Rename workspace"
                  bind:value={renameDraft}
                  class="workspace-rename-input"
                  on:blur={commitRenameWorkspace}
                  on:keydown={handleRenameKeydown}
                  autofocus
                />
              {:else}
                <button
                  aria-label={`Rename workspace ${workspace.name}`}
                  class="workspace-name"
                  disabled={disabled}
                  on:click={() => startRenameWorkspace(workspace.name)}
                  title="Click to rename"
                  type="button"
                >
                  {workspace.name}
                </button>
              {/if}
              <span class={`badge workspace-status ${workspace.phase}`}>
                <span class={`status-lamp ${workspace.phase}`}></span>
                {workspace.phase}
              </span>
              <span class="workspace-meta muted">
                {workspace.projects.length} project{workspace.projects.length === 1 ? "" : "s"}
                {#if workspace.runningCount > 0 && workspace.phase !== "running"}
                  · {workspace.runningCount} running
                {/if}
              </span>
            </div>
            <div class="actions workspace-actions">
              <button
                disabled={disabled}
                on:click={() => startWorkspace(workspace.projects)}
                type="button"
              >
                Start workspace
              </button>
              <button
                disabled={disabled}
                on:click={() => stopWorkspace(workspace.projects)}
                type="button"
              >
                Stop workspace
              </button>
              <button
                disabled={disabled}
                on:click={() => handleDeleteWorkspace(workspace.name, workspace.projects.length)}
                type="button"
              >
                Delete workspace
              </button>
            </div>
          </header>
          {#if isRenaming && renameError}
            <p class="project-error">{renameError}</p>
          {/if}
          {#if !collapsed}
            <div class="workspace-projects stack">
              {#if workspace.projects.length === 0}
                <p class="muted empty-workspace-hint">No projects in this workspace yet. Add one on the left, or move existing projects here from another workspace's "Move…" menu.</p>
              {/if}
              {#each workspace.projects as project (project.id)}
                {@const status = runtimeStatuses[project.id]}
                <article
                  class:selected={project.id === selectedProjectId}
                  class="project-card nested"
                  on:contextmenu={(e) => openProjectContextMenu(e, project)}
                  use:registerRow={project.id}
                >
                  <div class="project-row">
                    <div class="project-left">
                      <button class="select" on:click={() => onSelect(project.id)} type="button">
                        <h3>{project.name}</h3>
                      </button>
                      <p class="path" title={project.projectPath}>{project.projectPath}</p>
                    </div>

                    <div class="project-right">
                      <div class="status-actions">
                        <button
                          aria-label={`Refresh status for ${project.name}`}
                          class="icon-refresh"
                          disabled={disabled}
                          on:click={() => onRefresh(project.id)}
                          title="Refresh status"
                          type="button"
                        >
                          ↻
                        </button>
                        <span class={`badge ${status?.phase ?? "stopped"}`}>
                          <span class={`status-lamp ${status?.phase ?? "stopped"}`}></span>
                          {status?.phase ?? "stopped"}
                        </span>
                      </div>
                      <div class="actions row-actions">
                        <button disabled={disabled} on:click={() => onStart(project.id)} type="button">
                          Start
                        </button>
                        <button disabled={disabled} on:click={() => onStop(project.id)} type="button">
                          Stop
                        </button>
                        <button disabled={disabled} on:click={() => openMoveDropdown(project)} type="button">
                          Move…
                        </button>
                        <button disabled={disabled} on:click={() => onDelete(project.id)} type="button">
                          Delete
                        </button>
                      </div>
                    </div>
                  </div>
                  {#if movingProjectId === project.id}
                    {@const otherWorkspaces = knownWorkspaces.filter((n) => n !== project.workspaceName)}
                    <div class="move-dropdown">
                      <span class="move-dropdown-label">Move to:</span>
                      <select bind:value={moveSelection} disabled={disabled}>
                        {#each otherWorkspaces as ws}
                          <option value={ws}>{ws}</option>
                        {/each}
                        <option value="__new__">+ New workspace…</option>
                      </select>
                      {#if moveSelection === "__new__"}
                        <input
                          bind:value={moveNewName}
                          disabled={disabled}
                          placeholder="New workspace name"
                          on:keydown={(e) => {
                            if (e.key === "Enter") { e.preventDefault(); commitMove(project); }
                            else if (e.key === "Escape") { e.preventDefault(); cancelMove(); }
                          }}
                        />
                      {/if}
                      <button disabled={disabled} on:click={() => commitMove(project)} type="button">Save</button>
                      <button disabled={disabled} on:click={cancelMove} type="button">Cancel</button>
                    </div>
                  {/if}
                  {#if projectErrors[project.id]}
                    <p class="project-error">{projectErrors[project.id]}</p>
                  {:else if extractProjectError(status)}
                    <p class="project-error">{extractProjectError(status)}</p>
                  {/if}
                </article>
              {/each}
            </div>
          {/if}
        </article>
      {/each}
    </div>
  {/if}
</section>

{#if contextMenu}
  <ContextMenu
    items={contextMenu.items}
    onClose={closeContextMenu}
    x={contextMenu.x}
    y={contextMenu.y}
  />
{/if}
