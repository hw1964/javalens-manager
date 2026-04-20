<script lang="ts">
  import type {
    ProjectRecord,
    RuntimeStatusRecord
  } from "../api/tauri";

  export let projects: ProjectRecord[] = [];
  export let runtimeStatuses: Record<string, RuntimeStatusRecord> = {};
  export let selectedProjectId: string | undefined;
  export let disabled = false;

  export let onSelect: (projectId: string) => void;
  export let onStart: (projectId: string) => void;
  export let onStop: (projectId: string) => void;
  export let onDelete: (projectId: string) => void;
  export let onRefresh: (projectId: string) => void;
  export let onStartAll: () => void;
  export let onDeleteAll: () => void;

  function handleDeleteAll() {
    if (confirm("Delete all projects and stop all runtimes?")) {
      onDeleteAll();
    }
  }
</script>

<section class="panel stack">
  <div>
    <h2>Managed Projects</h2>
    <p class="muted">
      Each entry maps to one JavaLens runtime over stdio, resolved through the manager service.
    </p>
  </div>
  <div class="project-list-toolbar">
    <button disabled={disabled || projects.length === 0} on:click={() => onStartAll()} type="button">
      Start all
    </button>
    <button disabled={disabled || projects.length === 0} on:click={handleDeleteAll} type="button">
      Delete all
    </button>
  </div>

  {#if projects.length === 0}
    <div class="empty-state">
      No projects registered yet. Configure JavaLens first, then add a Java project on the left.
    </div>
  {:else}
    <div class="stack list">
      {#each projects as project}
        {@const status = runtimeStatuses[project.id]}
        <article class:selected={project.id === selectedProjectId} class="project-card">
          <button class="select" on:click={() => onSelect(project.id)} type="button">
            <div>
              <h3>{project.name}</h3>
              <p class="path">{project.projectPath}</p>
            </div>
            <span class={`badge ${status?.phase ?? "stopped"}`}>
              <span class={`status-lamp ${status?.phase ?? "stopped"}`}></span>
              {status?.phase ?? "stopped"}
            </span>
          </button>

          <div class="meta">
            <span>Transport: {status?.transport ?? "stdio"}</span>
            <span>Runtime: {status?.runtimeLabel ?? "unknown"}</span>
            <span>Port: {project.assignedPort}</span>
            <span>Workspace: {status?.workspaceDir ?? "unknown"}</span>
          </div>

          <div class="actions">
            <button disabled={disabled} on:click={() => onStart(project.id)} type="button">
              Start
            </button>
            <button disabled={disabled} on:click={() => onStop(project.id)} type="button">
              Stop
            </button>
            <button disabled={disabled} on:click={() => onRefresh(project.id)} type="button">
              Refresh
            </button>
            <button disabled={disabled} on:click={() => onDelete(project.id)} type="button">
              Delete
            </button>
          </div>
        </article>
      {/each}
    </div>
  {/if}
</section>
