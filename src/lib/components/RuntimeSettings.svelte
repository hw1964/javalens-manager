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
  export let installedRuntime: ManagedRuntimeRecord | null | undefined = undefined;
  export let disabled = false;

  const dispatch = createEventDispatcher<{
    save: UpdateSettingsInput;
    download: void;
    refresh: void;
  }>();

  let updatePolicy: UpdatePolicy = "ask";
  let autoCheckForUpdates = true;
  let dataRoot = "";
  let portRangeStart = 11100;
  let portRangeEnd = 11199;
  let runtimeKind: "managed" | "localJar" = "managed";
  let localJarPath = "";

  $: if (settings) {
    updatePolicy = settings.updatePolicy;
    autoCheckForUpdates = settings.autoCheckForUpdates;
    dataRoot = settings.dataRoot;
    portRangeStart = settings.portRangeStart;
    portRangeEnd = settings.portRangeEnd;
    runtimeKind = settings.globalRuntimeSource.kind;
    if (settings.globalRuntimeSource.kind === "localJar") {
      localJarPath = settings.globalRuntimeSource.jarPath;
    }
  }

  async function chooseDataRoot() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select Manager Data Root directory"
    });

    if (typeof selected === "string") {
      dataRoot = selected;
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

  function handleSave() {
    dispatch("save", {
      updatePolicy,
      autoCheckForUpdates,
      dataRoot,
      portRangeStart,
      portRangeEnd,
      globalRuntimeSource:
        runtimeKind === "managed"
          ? {
              kind: "managed"
            }
          : {
              kind: "localJar",
              jarPath: localJarPath
            }
    });
  }
</script>

<section class="panel stack">
  <div>
    <h2>Global JavaLens Runtime</h2>
    <p class="muted">
      Configure the single JavaLens version and data root used by all projects.
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
  </div>

  <label class="field">
    <span>Manager Data Root</span>
    <div class="field-row">
      <input
        bind:value={dataRoot}
        disabled={disabled}
        placeholder="/path/to/manager/data/root"
        required
      />
      <button disabled={disabled} on:click={chooseDataRoot} type="button">Browse</button>
    </div>
  </label>

  <label class="field">
    <span>Global JavaLens Source</span>
    <select bind:value={runtimeKind} disabled={disabled}>
      <option disabled={!installedRuntime} value="managed">Managed runtime</option>
      <option value="localJar">Local JAR fallback</option>
    </select>
  </label>

  {#if runtimeKind === "managed"}
    <div class="field">
      <span>Using Managed JavaLens</span>
      {#if !installedRuntime}
        <p class="hint">No managed runtime installed yet. Download the latest release first, or use local JAR mode.</p>
      {:else}
        <p><strong>v{installedRuntime.version}</strong></p>
      {/if}
    </div>
  {/if}

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
    <span>Permitted port range</span>
    <div class="field-row">
      <input bind:value={portRangeStart} disabled={disabled} min="1024" step="1" type="number" />
      <input bind:value={portRangeEnd} disabled={disabled} min="1024" step="1" type="number" />
    </div>
    <p class="hint">Manager assigns one project port inside this range.</p>
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
