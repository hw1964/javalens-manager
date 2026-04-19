<script lang="ts">
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

  $: if (settings) {
    updatePolicy = settings.updatePolicy;
    autoCheckForUpdates = settings.autoCheckForUpdates;
    defaultManagedRuntimeVersion = settings.defaultManagedRuntimeVersion ?? "";
  }

  $: if (
    installedRuntimes.length > 0 &&
    defaultManagedRuntimeVersion &&
    !installedRuntimes.some((runtime) => runtime.version === defaultManagedRuntimeVersion)
  ) {
    defaultManagedRuntimeVersion = installedRuntimes[0]?.version ?? "";
  }

  function handleSave() {
    dispatch("save", {
      updatePolicy,
      autoCheckForUpdates,
      defaultManagedRuntimeVersion: defaultManagedRuntimeVersion || null
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
      <p class="muted">{settings?.toolsDir ?? "No tools dir yet"}</p>
    </article>
  </div>

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
