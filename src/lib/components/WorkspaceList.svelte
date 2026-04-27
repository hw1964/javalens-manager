<script lang="ts">
  import type { ProjectRecord, RuntimeStatusRecord } from "../api/tauri";

  export let projects: ProjectRecord[] = [];
  export let runtimeStatuses: Record<string, RuntimeStatusRecord> = {};
  export let activeWorkspaceName: string;
  export let disabled = false;
  export let onSelect: (workspaceName: string) => void;

  /** Unsaved name for the "+ New workspace" inline input. Bound to the
   * input only when isCreating === true. */
  let isCreating = false;
  let newName = "";

  /** Per-workspace summary computed from the project list. */
  $: workspaceSummary = (() => {
    const byName: Record<
      string,
      { count: number; running: number }
    > = {};
    const order: string[] = [];
    for (const project of projects) {
      const name = project.workspaceName || "workspace-default";
      if (!byName[name]) {
        order.push(name);
        byName[name] = { count: 0, running: 0 };
      }
      byName[name].count += 1;
      if (runtimeStatuses[project.id]?.phase === "running") {
        byName[name].running += 1;
      }
    }
    return order.map((name) => ({
      name,
      count: byName[name].count,
      running: byName[name].running,
      phase:
        byName[name].running === byName[name].count && byName[name].count > 0
          ? "running"
          : byName[name].running === 0
            ? "stopped"
            : "starting",
    }));
  })();

  function startCreate() {
    isCreating = true;
    newName = "";
  }

  function commitCreate() {
    const trimmed = newName.trim();
    if (trimmed.length === 0) {
      isCreating = false;
      return;
    }
    onSelect(trimmed);
    isCreating = false;
    newName = "";
  }

  function cancelCreate() {
    isCreating = false;
    newName = "";
  }

  function handleNewKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      commitCreate();
    } else if (event.key === "Escape") {
      event.preventDefault();
      cancelCreate();
    }
  }
</script>

<section class="panel stack workspace-list-panel">
  <div class="section-intro">
    <h2>Workspaces</h2>
    <p class="muted">Pick a workspace to add new projects to. Multiple projects in one workspace share a single MCP service.</p>
  </div>

  {#if workspaceSummary.length === 0 && !isCreating}
    <p class="muted empty-hint">No workspaces yet. Create one to get started.</p>
  {/if}

  <ul class="workspace-list">
    {#each workspaceSummary as ws (ws.name)}
      <li>
        <button
          class:active={ws.name === activeWorkspaceName}
          class="workspace-row"
          disabled={disabled}
          on:click={() => onSelect(ws.name)}
          type="button"
        >
          <span class={`status-lamp ${ws.phase}`}></span>
          <span class="workspace-row-name">{ws.name}</span>
          <span class="workspace-row-meta muted">
            {ws.count} project{ws.count === 1 ? "" : "s"}
            {#if ws.running > 0 && ws.phase !== "running"}
              · {ws.running} running
            {/if}
          </span>
        </button>
      </li>
    {/each}

    {#if isCreating}
      <li>
        <div class="workspace-row workspace-new-row">
          <span class="status-lamp stopped"></span>
          <input
            bind:value={newName}
            class="workspace-new-input"
            on:blur={commitCreate}
            on:keydown={handleNewKeydown}
            placeholder="New workspace name"
            autofocus
          />
        </div>
      </li>
    {/if}
  </ul>

  {#if !isCreating}
    <button class="workspace-add" disabled={disabled} on:click={startCreate} type="button">
      + New workspace…
    </button>
  {/if}
</section>
