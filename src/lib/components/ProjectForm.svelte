<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { createEventDispatcher, onMount } from "svelte";
  import {
    discoverWorkspaceProjects,
    importWorkspaceProjects,
    suggestNextPort,
    type AddProjectInput,
    type WorkspaceProjectCandidate
  } from "../api/tauri";

  export let disabled = false;
  export let suggestedPort: number | null | undefined = undefined;

  const dispatch = createEventDispatcher<{
    submit: AddProjectInput;
    imported: void;
  }>();

  let name = "";
  let projectPath = "";
  let assignedPort = "";
  let userTouchedPort = false;
  let lastSuggestedName = "";
  let workspaceFile = "";
  let candidates: WorkspaceProjectCandidate[] = [];
  let selectedPaths: string[] = [];
  let importMessage = "";
  let isImporting = false;

  $: canSubmit =
    name.trim().length > 0 &&
    projectPath.trim().length > 0 &&
    assignedPort.trim().length > 0;

  // Auto-fill assignedPort with the latest server-suggested port whenever the
  // user has not manually edited it. The reactive statement re-fires when
  // suggestedPort changes (e.g., after a successful add the parent's
  // suggestedPort shifts to the next free port), keeping the form in sync
  // and preventing a "port already in use" error on the second submit.
  $: if (!userTouchedPort && suggestedPort) {
    assignedPort = String(suggestedPort);
  }

  onMount(async () => {
    if (!suggestedPort) {
      try {
        assignedPort = String(await suggestNextPort());
      } catch {
        // keep empty if suggestion cannot be resolved yet
      }
    }
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
    if (!workspaceFile.trim()) {
      importMessage = "Choose a .code-workspace file first.";
      return;
    }
    try {
      candidates = await discoverWorkspaceProjects(workspaceFile.trim());
      selectedPaths = candidates.map((candidate) => candidate.projectPath);
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
        selectedPaths
      });
      importMessage = `Imported ${result.added.length} project(s).`;
      if (result.skipped.length > 0) {
        importMessage += ` Skipped ${result.skipped.length}.`;
      }
      candidates = [];
      selectedPaths = [];
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
      assignedPort: Number(assignedPort)
    });

    name = "";
    projectPath = "";
    // Re-arm the auto-fill: drop the just-submitted port and let the reactive
    // $: statement above pick up the parent's freshly-shifted suggestedPort
    // once the dashboard refresh completes. Without this, the form would
    // submit the second project with the stale port and fail with
    // "Port X is already in use".
    userTouchedPort = false;
    assignedPort = "";
  }
</script>

<form class="panel stack" on:submit|preventDefault={handleSubmit}>
  <section class="stack">
    <div class="section-intro">
      <h2>Register Project</h2>
      <p class="muted">
        Pick a Java project folder and assign a project port.
      </p>
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

    <label class="field">
      <span>Assigned port</span>
      <input
        bind:value={assignedPort}
        disabled={disabled}
        min="1024"
        step="1"
        type="number"
        required
        on:input={() => (userTouchedPort = true)}
      />
    </label>

    <button class:primary={!disabled && canSubmit} disabled={disabled || !canSubmit} type="submit">Save project</button>
  </section>

  <hr class="section-divider" />

  <section class="stack">
    <div class="section-intro">
      <h2>Import VSCode Workspace</h2>
      <p class="muted">Discover Maven/Gradle and Eclipse/PDE Java projects from a .code-workspace.</p>
    </div>

    <label class="field">
      <span>.code-workspace file</span>
      <div class="field-row">
        <input bind:value={workspaceFile} disabled={disabled || isImporting} placeholder="/path/to/workspace.code-workspace" />
        <button disabled={disabled || isImporting} on:click={chooseWorkspaceFile} type="button">Browse</button>
      </div>
    </label>

    <button
      class:primary={!disabled && !isImporting && workspaceFile.trim().length > 0}
      disabled={disabled || isImporting || workspaceFile.trim().length === 0}
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
        class="primary"
        disabled={disabled || isImporting || selectedPaths.length === 0}
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
