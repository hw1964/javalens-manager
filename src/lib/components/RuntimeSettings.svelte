<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { createEventDispatcher } from "svelte";
  import type {
    ManagedRuntimeRecord,
    ManagerSettings,
    ReleaseStatus,
    UpdateSettingsInput,
    UpdatePolicy
  } from "../api/tauri";

  export let settings: ManagerSettings | undefined;
  export let releaseStatus: ReleaseStatus | undefined;
  export let installedRuntimes: ManagedRuntimeRecord[] = [];
  export let disabled = false;

  const dispatch = createEventDispatcher<{
    save: UpdateSettingsInput;
    download: void;
    refresh: void;
  }>();

  let updatePolicy: UpdatePolicy = "ask";
  let autoCheckForUpdates = true;
  let defaultManagedRuntimeVersion = "";
  let toolsDir = "";

  $: if (settings) {
    updatePolicy = settings.updatePolicy;
    autoCheckForUpdates = settings.autoCheckForUpdates;
    defaultManagedRuntimeVersion = settings.defaultManagedRuntimeVersion ?? "";
    toolsDir = settings.toolsDir;
  }

  $: if (
    installedRuntimes.length > 0 &&
    defaultManagedRuntimeVersion &&
    !installedRuntimes.some((runtime) => runtime.version === defaultManagedRuntimeVersion)
  ) {
    defaultManagedRuntimeVersion = installedRuntimes[0]?.version ?? "";
  }

  async function chooseToolsDir() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select JavaLens install directory"
    });

    if (typeof selected === "string") {
      toolsDir = selected;
    }
  }

  function handleSave() {
    dispatch("save", {
      updatePolicy,
      autoCheckForUpdates,
      defaultManagedRuntimeVersion: defaultManagedRuntimeVersion || null,
      toolsDir
    });
  }
</script>

<section class="panel stack">
  <div>
    <h2>JavaLens Runtime</h2>
    <p class="muted">
      The manager checks upstream releases, stores cached runtimes, and chooses a default managed
      version for new projects.
    </p>
  </div>

  <div class="card-grid">
    <article class="info-card">
      <span class="label">Release status</span>
      <strong>{releaseStatus?.kind ?? "unknown"}</strong>
      <p class="muted">{releaseStatus?.detail ?? "No release information loaded yet."}</p>
    </article>
    <article class="info-card">
      <span class="label">Latest upstream</span>
      <strong>{releaseStatus?.latestVersion ?? "unknown"}</strong>
      <p class="muted">Checked: {releaseStatus?.checkedAt ?? "not checked yet"}</p>
    </article>
    <article class="info-card">
      <span class="label">Managed runtimes</span>
      <strong>{installedRuntimes.length}</strong>
    </article>
  </div>

  <label class="field">
    <span>Install directory</span>
    <div class="field-row">
      <input
        bind:value={toolsDir}
        disabled={disabled}
        placeholder="/path/to/tools/dir"
        required
      />
      <button disabled={disabled} on:click={chooseToolsDir} type="button">Browse</button>
    </div>
  </label>

  <label class="field">
    <span>Update policy</span>
    <select bind:value={updatePolicy} disabled={disabled}>
      <option value="ask">Ask before updating</option>
      <option value="always">Always keep latest</option>
    </select>
  </label>

  <label class="checkbox-row">
    <input bind:checked={autoCheckForUpdates} disabled={disabled} type="checkbox" />
    <span>Check upstream JavaLens release on dashboard load</span>
  </label>

  <label class="field">
    <span>Default managed runtime</span>
    <select bind:value={defaultManagedRuntimeVersion} disabled={disabled || installedRuntimes.length === 0}>
      {#if installedRuntimes.length === 0}
        <option value="">No managed runtime installed yet</option>
      {:else}
        {#each installedRuntimes as runtime}
          <option value={runtime.version}>{runtime.version}</option>
        {/each}
      {/if}
    </select>
  </label>

  <div class="actions">
    <button class="primary" disabled={disabled} on:click={handleSave} type="button">
      Save settings
    </button>
    <button disabled={disabled} on:click={() => dispatch("download")} type="button">
      {releaseStatus?.updateAvailable ? "Download update" : "Download latest"}
    </button>
    <button disabled={disabled} on:click={() => dispatch("refresh")} type="button">
      Refresh release info
    </button>
  </div>
</section>
