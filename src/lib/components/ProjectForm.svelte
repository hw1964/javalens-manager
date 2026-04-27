<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { createEventDispatcher, onMount } from "svelte";
  import {
    discoverWorkspaceProjects,
    importWorkspaceProjects,
    type AddProjectInput,
    type WorkspaceProjectCandidate
  } from "../api/tauri";

  export let disabled = false;
  /** Sprint 10 v0.10.4: workspace name suggested by the manager (e.g. an
   * existing workspace, so the form pre-fills it for "join existing"). */
  export let suggestedWorkspaceName: string | null | undefined = undefined;
  /** Sprint 10 v0.10.4: existing workspace names so the dropdown can offer
   * "join existing" choices. */
  export let existingWorkspaceNames: string[] = [];

  const dispatch = createEventDispatcher<{
    submit: AddProjectInput;
    imported: void;
  }>();

  let name = "";
  let projectPath = "";
  /** Workspace selection: either a name from the dropdown ("__new__" =
   * create a new one) or an existing workspace name. */
  let workspaceSelection = "__new__";
  /** When workspaceSelection === "__new__", this is the name being typed. */
  let newWorkspaceName = "";
  let userTouchedWorkspace = false;
  let lastSuggestedName = "";
  let workspaceFile = "";
  let candidates: WorkspaceProjectCandidate[] = [];
  let selectedPaths: string[] = [];
  let importMessage = "";
  let isImporting = false;
  let lastDiscoveredFile = "";

  $: canDiscover =
    !disabled &&
    !isImporting &&
    workspaceFile.trim().length > 0 &&
    workspaceFile.trim() !== lastDiscoveredFile;

  $: canImportSelected =
    !disabled && !isImporting && selectedPaths.length > 0;

  /** The resolved workspace name from the form's current selection. */
  $: resolvedWorkspaceName =
    workspaceSelection === "__new__"
      ? newWorkspaceName.trim() || "workspace-default"
      : workspaceSelection;

  $: canSubmit =
    name.trim().length > 0 &&
    projectPath.trim().length > 0 &&
    resolvedWorkspaceName.length > 0;

  // Auto-fill workspace selection from the manager's suggestion until the
  // user touches the field. If a suggestion exists, prefer "join existing".
  $: if (!userTouchedWorkspace && suggestedWorkspaceName) {
    if (existingWorkspaceNames.includes(suggestedWorkspaceName)) {
      workspaceSelection = suggestedWorkspaceName;
    } else {
      newWorkspaceName = suggestedWorkspaceName;
    }
  }

  onMount(() => {
    /* no-op: workspace selection has sensible defaults */
  });

  function inferNameFromPath(path: string): string {
    const trimmed = path.trim().replace(/[\\/]+$/, "");
    if (!trimmed) {
      return "";
    }

    const parts = trimmed.split(/[\\/]/);
    return parts[parts.length - 1] ?? "";
  }

  function maybeAdoptSuggestedName(projectFolderName: string) {
    if (!projectFolderName) {
      return;
    }

    if (!name.trim() || name.trim() === lastSuggestedName) {
      name = projectFolderName;
      lastSuggestedName = projectFolderName;
    }
  }

  async function chooseProjectFolder() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select Java project folder"
    });

    if (typeof selected === "string") {
      projectPath = selected;
      maybeAdoptSuggestedName(inferNameFromPath(selected));
    }
  }

  async function chooseWorkspaceFile() {
    const selected = await open({
      directory: false,
      multiple: false,
      title: "Select VSCode workspace file",
      filters: [{ name: "VSCode Workspace", extensions: ["code-workspace"] }]
    });
    if (typeof selected === "string") {
      workspaceFile = selected;
    }
  }

  async function discoverFromWorkspace() {
    importMessage = "";
    const path = workspaceFile.trim();
    if (!path) {
      importMessage = "Choose a .code-workspace file first.";
      return;
    }
    try {
      candidates = await discoverWorkspaceProjects(path);
      selectedPaths = candidates.map((candidate) => candidate.projectPath);
      lastDiscoveredFile = path;
      if (candidates.length === 0) {
        importMessage = "No Maven/Gradle or Eclipse/PDE Java projects found.";
      }
    } catch (error) {
      importMessage = String(error);
    }
  }

  function toggleCandidate(path: string) {
    if (selectedPaths.includes(path)) {
      selectedPaths = selectedPaths.filter((value) => value !== path);
    } else {
      selectedPaths = [...selectedPaths, path];
    }
  }

  async function importSelected() {
    if (!workspaceFile.trim() || selectedPaths.length === 0) {
      importMessage = "Select at least one discovered project.";
      return;
    }
    isImporting = true;
    importMessage = "";
    try {
      const result = await importWorkspaceProjects({
        workspaceFile: workspaceFile.trim(),
        selectedPaths,
        workspaceName: resolvedWorkspaceName
      });
      importMessage = `Imported ${result.added.length} project(s).`;
      if (result.skipped.length > 0) {
        importMessage += ` Skipped ${result.skipped.length}.`;
      }
      candidates = [];
      selectedPaths = [];
      // Return the import section to its initial empty state so the buttons
      // grey out and the form is ready for the next operation.
      workspaceFile = "";
      lastDiscoveredFile = "";
      dispatch("imported");
    } catch (error) {
      importMessage = String(error);
    } finally {
      isImporting = false;
    }
  }

  function handleSubmit() {
    dispatch("submit", {
      name,
      projectPath,
      workspaceName: resolvedWorkspaceName
    });

    name = "";
    projectPath = "";
    // Re-arm auto-suggestion. Keep the workspace selection sticky (the
    // user is likely adding multiple projects to the same workspace).
    userTouchedWorkspace = false;
  }
</script>

<form class="panel stack" on:submit|preventDefault={handleSubmit}>
  <section class="stack">
    <div class="section-intro">
      <h2>Register Project</h2>
      <p class="muted">
        Pick a workspace, then a Java project folder.
      </p>
    </div>

    <div class="workspace-picker">
      <label class="field">
        <span>Workspace</span>
        <select
          bind:value={workspaceSelection}
          disabled={disabled}
          on:change={() => (userTouchedWorkspace = true)}
        >
          <option value="__new__">+ New workspace…</option>
          {#each existingWorkspaceNames as ws}
            <option value={ws}>{ws}</option>
          {/each}
        </select>
      </label>

      {#if workspaceSelection === "__new__"}
        <label class="field">
          <span>New workspace name</span>
          <input
            bind:value={newWorkspaceName}
            disabled={disabled}
            placeholder="workspace-default"
            on:input={() => (userTouchedWorkspace = true)}
          />
        </label>
      {/if}
    </div>

    <label class="field">
      <span>Name</span>
      <input bind:value={name} disabled={disabled} placeholder="Defaults to the selected folder name" required />
    </label>

    <label class="field">
      <span>Project path</span>
      <div class="field-row">
        <input
          bind:value={projectPath}
          disabled={disabled}
          placeholder="/path/to/java/project"
          required
        />
        <button disabled={disabled} on:click={chooseProjectFolder} type="button">Browse</button>
      </div>
    </label>

    <button class:primary={!disabled && canSubmit} disabled={disabled || !canSubmit} type="submit">Save project</button>
  </section>

  <hr class="section-divider" />

  <section class="stack">
    <div class="section-intro">
      <h2>Import from VSCode Workspace</h2>
      <p class="muted">Discover Maven/Gradle and Eclipse/PDE Java projects from a .code-workspace file. The selected projects join the workspace chosen above.</p>
    </div>

    <label class="field">
      <span>.code-workspace file</span>
      <div class="field-row">
        <input bind:value={workspaceFile} disabled={disabled || isImporting} placeholder="/path/to/workspace.code-workspace" />
        <button disabled={disabled || isImporting} on:click={chooseWorkspaceFile} type="button">Browse</button>
      </div>
    </label>

    <button
      class:primary={canDiscover}
      disabled={!canDiscover}
      on:click={discoverFromWorkspace}
      type="button"
    >
      Discover
    </button>

    {#if candidates.length > 0}
      <div class="stack candidate-list">
        {#each candidates as candidate}
          <label class="checkbox-row">
            <input
              checked={selectedPaths.includes(candidate.projectPath)}
              disabled={disabled || isImporting}
              on:change={() => toggleCandidate(candidate.projectPath)}
              type="checkbox"
            />
            <span>{candidate.name} ({candidate.kind}) - {candidate.projectPath}</span>
          </label>
        {/each}
      </div>
      <button
        class:primary={canImportSelected}
        disabled={!canImportSelected}
        on:click={importSelected}
        type="button"
      >
        Import selected
      </button>
    {/if}

    {#if importMessage}
      <p class="muted">{importMessage}</p>
    {/if}
  </section>
</form>
