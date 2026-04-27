<script lang="ts">
  import type { ProjectRecord, RuntimeStatusRecord } from "../api/tauri";

  export let projects: ProjectRecord[] = [];
  export let runtimeStatuses: Record<string, RuntimeStatusRecord> = {};
  export let activeWorkspaceName: string;
  export let disabled = false;
  export let onSelect: (workspaceName: string) => void;
  /** Optional: rename a workspace on the left. When omitted the icon
   * is hidden. */
  export let onRename: ((oldName: string, newName: string) => void) | undefined = undefined;
  /** Optional: delete a workspace on the left. When omitted the icon
   * is hidden. */
  export let onDelete: ((name: string) => void) | undefined = undefined;
  /** All workspaces the UI knows about, including pinned empty ones
   * (newly-created via "+ New workspace…" with no projects yet).
   * Sorted, deduped — owner is App.svelte. */
  export let knownWorkspaces: string[] = [];

  /** Unsaved name for the "+ New workspace" inline input. Bound to the
   * input only when isCreating === true. */
  let isCreating = false;
  let newName = "";

  type Phase = "running" | "stopped" | "starting";
  /** Reduce a (count, running) tally to the workspace's aggregate phase.
   * Empty (count=0) → stopped; all running → running; none running →
   * stopped; mixed → starting. */
  function derivePhase(count: number, running: number): Phase {
    if (count === 0) return "stopped";
    if (running === count) return "running";
    if (running === 0) return "stopped";
    return "starting";
  }

  /** Per-workspace summary derived from the union of `knownWorkspaces`
   * and `projects`: every known workspace renders a row, even if it has
   * no projects yet. */
  $: workspaceSummary = (() => {
    const byName: Record<
      string,
      { count: number; running: number }
    > = {};
    for (const name of knownWorkspaces) {
      byName[name] = { count: 0, running: 0 };
    }
    for (const project of projects) {
      const name = project.workspaceName || "workspace-default";
      if (!byName[name]) {
        byName[name] = { count: 0, running: 0 };
      }
      byName[name].count += 1;
      if (runtimeStatuses[project.id]?.phase === "running") {
        byName[name].running += 1;
      }
    }
    return Object.keys(byName)
      .sort()
      .map((name) => ({
        name,
        count: byName[name].count,
        running: byName[name].running,
        phase: derivePhase(byName[name].count, byName[name].running),
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

  function promptRename(name: string) {
    if (!onRename) return;
    const next = window.prompt(`Rename workspace "${name}" to:`, name);
    if (next && next.trim().length > 0 && next.trim() !== name) {
      onRename(name, next.trim());
    }
  }

  function confirmDelete(name: string, count: number) {
    if (!onDelete) return;
    const detail =
      count === 0
        ? `Delete workspace "${name}"?`
        : `Delete workspace "${name}" and all ${count} project(s) inside it?`;
    if (window.confirm(detail)) {
      onDelete(name);
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
        <div
          class:active={ws.name === activeWorkspaceName}
          class="workspace-row"
        >
          <button
            class="workspace-row-select"
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
          <span class="workspace-row-tools">
            {#if onRename}
              <button
                aria-label={`Rename ${ws.name}`}
                class="workspace-row-icon"
                disabled={disabled}
                on:click|stopPropagation={() => promptRename(ws.name)}
                title="Rename workspace"
                type="button"
              >
                ✎
              </button>
            {/if}
            {#if onDelete}
              <button
                aria-label={`Delete ${ws.name}`}
                class="workspace-row-icon danger"
                disabled={disabled}
                on:click|stopPropagation={() => confirmDelete(ws.name, ws.count)}
                title="Delete workspace"
                type="button"
              >
                ✕
              </button>
            {/if}
          </span>
        </div>
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
