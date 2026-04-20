<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { createEventDispatcher } from "svelte";
  import type {
    BootstrapStatus,
    CleanupSummary,
    ManagedRuntimeRecord,
    McpClientPathEntry,
    McpClientPaths,
    McpMergeMode,
    ManagerSettings,
    ReleaseStatus,
    ServiceProbeResult,
    UpdateSettingsInput,
    UpdatePolicy
  } from "../api/tauri";

  export let settings: ManagerSettings | undefined;
  export let releaseStatus: ReleaseStatus | undefined;
  export let installedRuntime: ManagedRuntimeRecord | null | undefined = undefined;
  export let bootstrap: BootstrapStatus | undefined;
  export let lastCleanupSummary: CleanupSummary | undefined;
  export let lastServiceProbe: ServiceProbeResult | undefined;
  export let serviceProbeBusy = false;
  export let serviceProbeError: string | undefined;
  export let disabled = false;

  const dispatch = createEventDispatcher<{
    save: UpdateSettingsInput;
    download: void;
    refresh: void;
    cleanLogs: void;
    cleanWorkspaces: void;
    cleanGeneratedData: void;
    clearCleanupSummary: void;
    probeServices: void;
    clearServiceProbeError: void;
  }>();

  let updatePolicy: UpdatePolicy = "ask";
  let autoCheckForUpdates = true;
  let dataRoot = "";
  let portRangeStart = 11100;
  let portRangeEnd = 11199;
  let useSystemTray = true;
  let runtimeKind: "managed" | "localJar" = "managed";
  let localJarPath = "";
  let mcpMergeMode: McpMergeMode = "safeMerge";
  let mcpBackupBeforeWrite = true;
  let mcpClientPaths: McpClientPaths = {
    cursor: {},
    claude: {},
    antigravity: {},
    intellij: {}
  };

  $: if (settings) {
    updatePolicy = settings.updatePolicy;
    autoCheckForUpdates = settings.autoCheckForUpdates;
    dataRoot = settings.dataRoot;
    portRangeStart = settings.portRangeStart;
    portRangeEnd = settings.portRangeEnd;
    useSystemTray = settings.useSystemTray;
    runtimeKind = settings.globalRuntimeSource.kind;
    mcpMergeMode = settings.mcpMergeMode;
    mcpBackupBeforeWrite = settings.mcpBackupBeforeWrite;
    mcpClientPaths = settings.mcpClientPaths;
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

  async function chooseMcpPath(client: keyof McpClientPaths) {
    const selected = await open({
      directory: false,
      multiple: false,
      title: `Select ${client} MCP config file`
    });
    if (typeof selected === "string") {
      setManualMcpPath(client, selected);
    }
  }

  function setManualMcpPath(client: keyof McpClientPaths, path: string) {
    mcpClientPaths = {
      ...mcpClientPaths,
      [client]: {
        ...(mcpClientPaths[client] ?? {}),
        manualOverridePath: path
      }
    };
  }

  function clearManualMcpPath(client: keyof McpClientPaths) {
    mcpClientPaths = {
      ...mcpClientPaths,
      [client]: {
        ...(mcpClientPaths[client] ?? {}),
        manualOverridePath: null
      }
    };
  }

  function confirmAndDispatch(
    message: string,
    eventName: "cleanLogs" | "cleanWorkspaces" | "cleanGeneratedData"
  ) {
    if (confirm(message)) {
      dispatch(eventName);
    }
  }

  function effectivePath(entry: McpClientPathEntry | undefined): string {
    return entry?.effectivePath ?? entry?.manualOverridePath ?? entry?.autoDetectedPath ?? "not configured";
  }

  function handleSave() {
    dispatch("save", {
      updatePolicy,
      autoCheckForUpdates,
      dataRoot,
      portRangeStart,
      portRangeEnd,
      useSystemTray,
      globalRuntimeSource:
        runtimeKind === "managed"
          ? {
              kind: "managed"
            }
          : {
              kind: "localJar",
              jarPath: localJarPath
            },
      mcpClientPaths,
      mcpMergeMode,
      mcpBackupBeforeWrite
    });
  }
</script>

<section class="panel stack">
  <div>
    <h2>Settings</h2>
    <p class="muted">Configure JavaLens runtime, machine controls, diagnostics, and MCP location metadata.</p>
  </div>

  <div class="settings-grid">
    <section class="panel stack settings-section runtime-section">
      <div class="section-intro">
        <h3>JavaLens Runtime</h3>
        <p class="muted">Runtime updates and source selection.</p>
      </div>
      <div class="runtime-summary">
        <span class="runtime-chip">
          Status: <strong>{releaseStatus?.kind ?? "unknown"}</strong>
        </span>
        <span class="runtime-chip">
          Latest: <strong>{releaseStatus?.latestVersion ?? "unknown"}</strong>
        </span>
        <span class="runtime-chip">
          Checked: <strong>{releaseStatus?.checkedAt ?? "n/a"}</strong>
        </span>
      </div>

      <label class="field">
        <span>Global JavaLens Source</span>
        <select bind:value={runtimeKind} disabled={disabled}>
          <option disabled={!installedRuntime} value="managed">Managed runtime</option>
          <option value="localJar">Local JAR fallback</option>
        </select>
      </label>

      {#if runtimeKind === "managed"}
        <p class="hint">
          Active: {#if !installedRuntime}not installed{:else}<strong>v{installedRuntime.version}</strong>{/if}
        </p>
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
        <button disabled={disabled} on:click={() => dispatch("download")} type="button">
          {releaseStatus?.updateAvailable ? "Download update" : "Download latest"}
        </button>
        <button disabled={disabled} on:click={() => dispatch("refresh")} type="button">
          Refresh release info
        </button>
      </div>
    </section>

    <section class="panel stack settings-section">
      <div class="section-intro">
        <h3>Exposed Services</h3>
        <p class="muted">Probe JavaLens live to detect exposed MCP tools.</p>
      </div>
      {#if !lastServiceProbe && !serviceProbeBusy}
        <p class="hint">Click <strong>Test Services</strong> to run a live MCP handshake and list tools.</p>
      {/if}
      <div class="actions">
        <button
          disabled={disabled || serviceProbeBusy}
          on:click={() => dispatch("probeServices")}
          type="button"
        >
          {serviceProbeBusy ? "Testing..." : "Test Services"}
        </button>
      </div>
      {#if lastServiceProbe}
        <div class={`probe-result ${lastServiceProbe.ok ? "ok" : "error"}`}>
          {#if lastServiceProbe.ok}
            <p><strong>Probe successful</strong></p>
            <p class="hint">Duration: {lastServiceProbe.durationMs} ms</p>
            <p class="hint">Found {lastServiceProbe.services.length} services</p>
          {:else}
            <p>{lastServiceProbe.detail}</p>
            <p class="hint">Duration: {lastServiceProbe.durationMs} ms</p>
          {/if}
          {#if lastServiceProbe.ok && lastServiceProbe.services.length > 0}
            <ul class="service-list compact service-list-scroll">
              {#each lastServiceProbe.services as service}
                <li class="service-item">
                  <strong title={service.name}>{service.name}</strong>
                  <span class="hint" title={service.description ?? "No description provided by JavaLens."}>
                    {service.description ?? "No description provided by JavaLens."}
                  </span>
                </li>
              {/each}
            </ul>
          {/if}
          {#if !lastServiceProbe.ok && lastServiceProbe.rawProtocolError}
            <p class="hint">Protocol error: {lastServiceProbe.rawProtocolError}</p>
          {/if}
        </div>
      {/if}
      {#if serviceProbeError && !lastServiceProbe}
        <div class="probe-result error">
          <p>{serviceProbeError}</p>
          <div class="actions">
            <button on:click={() => dispatch("clearServiceProbeError")} type="button">Dismiss</button>
          </div>
        </div>
      {/if}
    </section>

    <section class="panel stack settings-section">
      <div class="section-intro">
        <h3>Machine Runtime Controls</h3>
        <p class="muted">Machine-local paths and networking controls.</p>
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
        <span>Permitted port range</span>
        <div class="field-row">
          <input bind:value={portRangeStart} disabled={disabled} min="1024" step="1" type="number" />
          <input bind:value={portRangeEnd} disabled={disabled} min="1024" step="1" type="number" />
        </div>
        <p class="hint">Manager assigns one project port inside this range and validates conflicts.</p>
      </label>

      <label class="checkbox-row">
        <input bind:checked={useSystemTray} disabled={disabled} type="checkbox" />
        <span>Use system tray for manager background visibility</span>
      </label>
    </section>

    <section class="panel stack settings-section">
      <div class="section-intro">
        <h3>MCP Config Locations</h3>
        <p class="muted">Store defaults and manual overrides here. Deploy execution is handled in Sprint 7 workflow.</p>
      </div>

      {#each [
        ["cursor", "Cursor"],
        ["claude", "Claude"],
        ["antigravity", "Antigravity"],
        ["intellij", "IntelliJ"]
      ] as [clientKey, clientLabel]}
        {@const key = clientKey as keyof McpClientPaths}
        {@const entry = mcpClientPaths[key]}
        <div class="stack mcp-client-card">
          <strong>{clientLabel}</strong>
          <p class="hint">Auto-detected: {entry?.autoDetectedPath ?? "not found"}</p>
          <p class="hint">Effective: {effectivePath(entry)}</p>
          <div class="field-row">
            <input
              disabled={disabled}
              on:input={(event) => setManualMcpPath(key, (event.currentTarget as HTMLInputElement).value)}
              placeholder="Manual override path"
              value={entry?.manualOverridePath ?? ""}
            />
            <button disabled={disabled} on:click={() => chooseMcpPath(key)} type="button">Browse</button>
            <button disabled={disabled} on:click={() => clearManualMcpPath(key)} type="button">Clear</button>
          </div>
        </div>
      {/each}

      <label class="field">
        <span>Merge mode</span>
        <select bind:value={mcpMergeMode} disabled={disabled}>
          <option value="safeMerge">Safe merge</option>
          <option value="replaceManagedSection">Replace managed section</option>
        </select>
      </label>

      <label class="checkbox-row">
        <input bind:checked={mcpBackupBeforeWrite} disabled={disabled} type="checkbox" />
        <span>Create backup before MCP config write</span>
      </label>
    </section>

    <section class="panel stack settings-section">
      <div class="section-intro">
        <h3>Diagnostics &amp; Reset</h3>
        <p class="muted">Paths and safe cleanup actions for starting from scratch.</p>
      </div>
      <div class="bootstrap-grid">
        <div>
          <span class="label">Projects</span>
          <strong>{bootstrap?.projectsFile ?? "-"}</strong>
        </div>
        <div>
          <span class="label">Settings</span>
          <strong>{bootstrap?.settingsFile ?? "-"}</strong>
        </div>
        <div>
          <span class="label">State</span>
          <strong>{bootstrap?.stateDir ?? "-"}</strong>
        </div>
        <div>
          <span class="label">Data root</span>
          <strong>{bootstrap?.defaultDataRoot ?? "-"}</strong>
        </div>
      </div>

      <div class="actions">
        <button
          disabled={disabled}
          on:click={() =>
            confirmAndDispatch(
              "Delete all manager runtime logs? This keeps project registrations and settings.",
              "cleanLogs"
            )}
          type="button"
        >
          Clean logs
        </button>
        <button
          disabled={disabled}
          on:click={() =>
            confirmAndDispatch(
              "Delete all manager workspace/index caches? This keeps project registrations and settings.",
              "cleanWorkspaces"
            )}
          type="button"
        >
          Clean workspaces
        </button>
        <button
          disabled={disabled}
          on:click={() =>
            confirmAndDispatch(
              "Start from scratch by deleting generated logs + workspaces? Stop running runtimes first.",
              "cleanGeneratedData"
            )}
          type="button"
        >
          Start from scratch
        </button>
      </div>

      {#if lastCleanupSummary}
        <div class="banner">
          <span>{lastCleanupSummary.detail} Files: {lastCleanupSummary.deletedFiles}, Dirs: {lastCleanupSummary.deletedDirs}</span>
          <button on:click={() => dispatch("clearCleanupSummary")} type="button">Dismiss</button>
        </div>
      {/if}
    </section>
  </div>

  <div class="actions">
    <button class="primary" disabled={disabled} on:click={handleSave} type="button">
      Save settings
    </button>
  </div>
</section>
