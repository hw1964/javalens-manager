<script lang="ts">
  import type {
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
  export let onUpdatePort: (projectId: string, assignedPort: number) => void;

  let draftPorts: Record<string, string> = {};
  let portInputErrors: Record<string, string> = {};

  function handleDeleteAll() {
    if (confirm("Delete all projects and stop all runtimes?")) {
      onDeleteAll();
    }
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

  function getDraftPort(project: ProjectRecord): string {
    return draftPorts[project.id] ?? String(project.assignedPort);
  }

  function updateDraftPort(projectId: string, value: string) {
    draftPorts = {
      ...draftPorts,
      [projectId]: value
    };
    if (portInputErrors[projectId]) {
      const nextErrors = { ...portInputErrors };
      delete nextErrors[projectId];
      portInputErrors = nextErrors;
    }
  }

  function applyPort(project: ProjectRecord) {
    const raw = (draftPorts[project.id] ?? String(project.assignedPort)).trim();
    const parsed = Number(raw);
    if (!Number.isInteger(parsed) || parsed < 1024 || parsed > 65535) {
      portInputErrors = {
        ...portInputErrors,
        [project.id]: "Port must be an integer between 1024 and 65535."
      };
      return;
    }
    onUpdatePort(project.id, parsed);
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

  {#if projects.length === 0}
    <div class="empty-state">
      No projects registered yet. Configure JavaLens first, then add a Java project on the left.
    </div>
  {:else}
    <div class="stack list project-list-scroll">
      {#each projects as project}
        {@const status = runtimeStatuses[project.id]}
        <article class:selected={project.id === selectedProjectId} class="project-card">
          <div class="project-row">
            <div class="project-left">
              <button class="select" on:click={() => onSelect(project.id)} type="button">
                <h3>{project.name}</h3>
              </button>
              <p class="path" title={project.projectPath}>{project.projectPath}</p>
              <div class="meta">
                <div class="port-row">
                  <span>Port</span>
                  <div class="port-editor">
                    <input
                      aria-label={`Assigned port for ${project.name}`}
                      disabled={disabled}
                      inputmode="numeric"
                      on:input={(event) => updateDraftPort(project.id, (event.currentTarget as HTMLInputElement).value)}
                      value={getDraftPort(project)}
                    />
                    <button disabled={disabled} on:click={() => applyPort(project)} type="button">
                      Set
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
          {#if portInputErrors[project.id]}
            <p class="project-error">{portInputErrors[project.id]}</p>
          {:else if projectErrors[project.id]}
            <p class="project-error">{projectErrors[project.id]}</p>
          {:else if extractProjectError(status)}
            <p class="project-error">{extractProjectError(status)}</p>
          {/if}
        </article>
      {/each}
    </div>
  {/if}
</section>
