<script lang="ts">
  import { afterUpdate } from "svelte";
  import type {
    DeployTargetFlags,
    DeployMode,
    DeployToAgentsResult,
    ProjectRecord,
    RuntimeStatusRecord
  } from "../api/tauri";

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

  /** Sprint 10 v0.10.4: prompt for a new workspace name and move the
   * project. The minimal UI for this — a button on the project row that
   * opens a prompt() — is replaced by the grouped Dashboard view in a
   * follow-up; this stub keeps the wiring for the new API. */
  function moveProjectToWorkspace(project: ProjectRecord) {
    const target = window.prompt(
      `Move "${project.name}" to which workspace?\nCurrently in: ${project.workspaceName}`,
      project.workspaceName
    );
    if (target && target.trim().length > 0 && target.trim() !== project.workspaceName) {
      onSetWorkspace(project.id, target.trim());
    }
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
      <p class="muted">Project services managed by the manager.</p>
      <div class="project-summary-metrics">
        <span class="metric-pill">Total {totalProjects}</span>
        <span class="metric-pill running">Running {runningProjects}</span>
        <span class="metric-pill stopped">Not running {stoppedProjects}</span>
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
      {#each projects as project}
        {@const status = runtimeStatuses[project.id]}
        <article
          class:selected={project.id === selectedProjectId}
          class="project-card"
          use:registerRow={project.id}
        >
          <div class="project-row">
            <div class="project-left">
              <button class="select" on:click={() => onSelect(project.id)} type="button">
                <h3>{project.name}</h3>
              </button>
              <p class="path" title={project.projectPath}>{project.projectPath}</p>
              <div class="meta">
                <div class="port-row">
                  <span>Workspace</span>
                  <div class="port-editor">
                    <span title={`Workspace ${project.workspaceName}`}>{project.workspaceName}</span>
                    <button
                      disabled={disabled}
                      on:click={() => moveProjectToWorkspace(project)}
                      type="button"
                    >
                      Move…
                    </button>
                  </div>
                </div>
              </div>
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
                <button disabled={disabled} on:click={() => onDelete(project.id)} type="button">
                  Delete
                </button>
              </div>
            </div>
          </div>
          {#if projectErrors[project.id]}
            <p class="project-error">{projectErrors[project.id]}</p>
          {:else if extractProjectError(status)}
            <p class="project-error">{extractProjectError(status)}</p>
          {/if}
        </article>
      {/each}
    </div>
  {/if}
</section>
