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
  /** Sprint 10 v0.10.4: the workspace this form's submit will add the
   * project to. Owned by the parent (App.svelte) and shared with the
   * Workspaces card on the left so that picking a workspace there
   * routes new projects (and imports) to it. Empty string = no
   * workspace selected yet. */
  export let activeWorkspaceName: string = "";

  const dispatch = createEventDispatcher<{
    submit: AddProjectInput;
    imported: void;
  }>();

  let name = "";
  let projectPath = "";
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
    !disabled && !isImporting && selectedPaths.length > 0 && activeWorkspaceName.length > 0;

  $: canSubmit =
    name.trim().length > 0 &&
    projectPath.trim().length > 0 &&
    activeWorkspaceName.length > 0;

  onMount(() => {
    /* no-op */
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
        workspaceName: activeWorkspaceName
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
      workspaceName: activeWorkspaceName
    });

    name = "";
    projectPath = "";
    // activeWorkspaceName persists across submits — owned by the parent
    // and shared with the Workspaces card. The user is likely adding
    // multiple projects to the same workspace.
  }
</script>

<form class="panel stack" on:submit|preventDefault={handleSubmit}>
  <section class="stack">
    <div class="section-intro">
      <h2>Register Project</h2>
      <p class="muted">
        {#if activeWorkspaceName}
          Adding to <strong>{activeWorkspaceName}</strong>. Pick a different workspace above to change.
        {:else}
          Pick a workspace above first.
        {/if}
      </p>
    </div>

    <label class="field">
      <span>Name</span>
      <input bind:value={name} disabled={disabled || !activeWorkspaceName} placeholder="Defaults to the selected folder name" required />
    </label>

    <label class="field">
      <span>Project path</span>
      <div class="field-row">
        <input
          bind:value={projectPath}
          disabled={disabled || !activeWorkspaceName}
          placeholder="/path/to/java/project"
          required
        />
        <button disabled={disabled || !activeWorkspaceName} on:click={chooseProjectFolder} type="button">Browse</button>
      </div>
    </label>

    <button class:primary={!disabled && canSubmit} disabled={disabled || !canSubmit} type="submit">Save project</button>
  </section>

  <hr class="section-divider" />

  <section class="stack">
    <div class="section-intro">
      <h2>Import from VSCode Workspace</h2>
      <p class="muted">
        {#if activeWorkspaceName}
          Discover Maven/Gradle and Eclipse/PDE Java projects from a .code-workspace file. Selected projects join <strong>{activeWorkspaceName}</strong>.
        {:else}
          Pick a workspace above first.
        {/if}
      </p>
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
