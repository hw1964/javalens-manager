<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { createEventDispatcher } from "svelte";
  import type { AddProjectInput, ManagedRuntimeRecord } from "../api/tauri";

  export let disabled = false;
  export let installedRuntimes: ManagedRuntimeRecord[] = [];
  export let defaultManagedRuntimeVersion: string | null | undefined = undefined;

  const dispatch = createEventDispatcher<{
    submit: AddProjectInput;
  }>();

  let name = "";
  let projectPath = "";
  let workspaceDir = "";
  let runtimeKind: "managed" | "localJar" = "managed";
  let selectedManagedVersion = "";
  let localJarPath = "";
  let lastSuggestedName = "";
  let showAdvanced = false;

  $: if (installedRuntimes.length === 0 && runtimeKind === "managed") {
    runtimeKind = "localJar";
    showAdvanced = true;
  }

  $: if (runtimeKind === "managed") {
    const preferred =
      defaultManagedRuntimeVersion && installedRuntimes.some((runtime) => runtime.version === defaultManagedRuntimeVersion)
        ? defaultManagedRuntimeVersion
        : installedRuntimes[0]?.version ?? "";

    if (!selectedManagedVersion || !installedRuntimes.some((runtime) => runtime.version === selectedManagedVersion)) {
      selectedManagedVersion = preferred;
    }
  }

  $: canSubmit =
    name.trim().length > 0 &&
    projectPath.trim().length > 0 &&
    (runtimeKind === "managed"
      ? selectedManagedVersion.trim().length > 0
      : localJarPath.trim().length > 0);

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

  async function chooseLocalJar() {
    const selected = await open({
      directory: false,
      multiple: false,
      title: "Select JavaLens JAR",
      filters: [
        {
          name: "Java archive",
          extensions: ["jar"]
        }
      ]
    });

    if (typeof selected === "string") {
      localJarPath = selected;
    }
  }

  function handleSubmit() {
    dispatch("submit", {
      name,
      projectPath,
      workspaceDir: workspaceDir.trim() || undefined,
      runtimeSource:
        runtimeKind === "managed"
          ? {
              kind: "managed",
              version: selectedManagedVersion
            }
          : {
              kind: "localJar",
              jarPath: localJarPath
            }
    });

    name = "";
    projectPath = "";
    workspaceDir = "";
    localJarPath = "";
    showAdvanced = false;
  }
</script>

<form class="panel stack" on:submit|preventDefault={handleSubmit}>
  <div class="section-intro">
    <h2>Register Project</h2>
    <p class="muted">
      Pick a Java project folder and bind it to a managed JavaLens runtime.
    </p>
  </div>

  <label class="field">
    <span>Name</span>
    <input
      bind:value={name}
      disabled={disabled}
      placeholder="Defaults to the selected folder name"
      required
    />
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

  {#if runtimeKind === "managed"}
    <label class="field">
      <span>Installed JavaLens version</span>
      <select bind:value={selectedManagedVersion} disabled={disabled || installedRuntimes.length === 0}>
        {#if installedRuntimes.length === 0}
          <option value="">No managed runtime installed yet</option>
        {:else}
          {#each installedRuntimes as runtime}
            <option value={runtime.version}>{runtime.version}</option>
          {/each}
        {/if}
      </select>
    </label>
    {#if installedRuntimes.length === 0}
      <p class="hint">Download the latest JavaLens release first in Settings, or use advanced options for local JAR mode.</p>
    {/if}
  {/if}

  <details class="advanced-toggle" bind:open={showAdvanced}>
    <summary>Advanced Options</summary>
    <div class="stack advanced-content">
      <label class="field">
        <span>JavaLens source</span>
        <select bind:value={runtimeKind} disabled={disabled}>
          <option disabled={installedRuntimes.length === 0} value="managed">Managed runtime</option>
          <option value="localJar">Local JAR fallback</option>
        </select>
      </label>

      {#if runtimeKind === "localJar"}
        <label class="field">
          <span>Local JavaLens JAR path</span>
          <div class="field-row">
            <input
              bind:value={localJarPath}
              disabled={disabled}
              placeholder="/path/to/javalens.jar"
              required={runtimeKind === "localJar"}
            />
            <button disabled={disabled} on:click={chooseLocalJar} type="button">Browse</button>
          </div>
        </label>
      {/if}

      <label class="field">
        <span>Workspace dir override</span>
        <input
          bind:value={workspaceDir}
          disabled={disabled}
          placeholder="Optional. Defaults to manager-owned cache path."
        />
      </label>
    </div>
  </details>

  <button class="primary" disabled={disabled || !canSubmit} type="submit">Save project</button>
</form>
