<script lang="ts">
  import { onMount } from "svelte";
  import ProjectForm from "./lib/components/ProjectForm.svelte";
  import ProjectList from "./lib/components/ProjectList.svelte";
  import RuntimeSettings from "./lib/components/RuntimeSettings.svelte";
  import { createAppStore } from "./lib/stores/app";
  import {
    type AddProjectInput,
    type UpdateSettingsInput
  } from "./lib/api/tauri";

  const appStore = createAppStore();
  const managerBuildVersion = "20260420.01";

  let currentView: "dashboard" | "settings" = "dashboard";

  $: selectedProject = $appStore.projects?.find((project) => project.id === $appStore.selectedProjectId);
  $: selectedStatus = selectedProject
    ? $appStore.runtimeStatuses?.[selectedProject.id]
    : undefined;
  $: runtimeSubtitle = $appStore.installedRuntime?.version
    ? `javalens-manager ${managerBuildVersion} | JavaLens ${$appStore.installedRuntime.version}${
        $appStore.releaseStatus?.updateAvailable
          ? ` (update: ${$appStore.releaseStatus.latestVersion ?? "available"})`
          : ""
      }`
    : `javalens-manager ${managerBuildVersion} | JavaLens runtime not downloaded`;

  onMount(() => {
    appStore.load();
  });

  function handleProjectSubmit(event: CustomEvent<AddProjectInput>) {
    appStore.addProjectEntry(event.detail);
  }

  function handleSettingsSave(event: CustomEvent<UpdateSettingsInput>) {
    appStore.updateManagerSettings(event.detail);
  }
</script>

<svelte:head>
  <title>javalens-manager</title>
</svelte:head>

<main class="app-shell">
  <header class="hero panel">
    <div class="header-content">
      <div>
        <h1>javalens-manager</h1>
        <p class="title-subline muted">{runtimeSubtitle}</p>
      </div>
      <nav class="nav-tabs">
        <button
          class="tab {currentView === 'dashboard' ? 'active' : ''}"
          on:click={() => (currentView = 'dashboard')}
          type="button"
        >
          Dashboard
        </button>
        <button
          class="tab {currentView === 'settings' ? 'active' : ''}"
          on:click={() => (currentView = 'settings')}
          type="button"
        >
          Settings
        </button>
      </nav>
    </div>
  </header>

  {#if $appStore.error}
    <div class="banner error">
      <span>{$appStore.error}</span>
      <button on:click={() => appStore.clearError()} type="button">Dismiss</button>
    </div>
  {/if}

  {#if currentView === 'dashboard'}
    <section class="layout dashboard-layout">
      <div class="dashboard-column">
        <ProjectForm
          disabled={$appStore.isBusy}
          suggestedPort={$appStore.suggestedPort}
          on:submit={handleProjectSubmit}
          on:imported={() => appStore.load()}
        />
      </div>

      <div class="dashboard-column">
        <ProjectList
          disabled={$appStore.isBusy}
          onRefresh={(projectId) => appStore.refreshProjectStatus(projectId)}
          onSelect={(projectId) => appStore.selectProject(projectId)}
          onStart={(projectId) => appStore.startProject(projectId)}
          onStartAll={() => appStore.startAllProjects()}
          onStop={(projectId) => appStore.stopProject(projectId)}
          onStopAll={() => appStore.stopAllProjects()}
          onDelete={(projectId) => appStore.deleteProjectEntry(projectId)}
          onDeleteAll={() => appStore.deleteAllProjectEntries()}
          onUpdatePort={(projectId, assignedPort) => appStore.updateProjectPortEntry(projectId, assignedPort)}
          projects={$appStore.projects ?? []}
          projectErrors={$appStore.projectErrors ?? {}}
          runtimeStatuses={$appStore.runtimeStatuses ?? {}}
          selectedProjectId={$appStore.selectedProjectId}
        />
      </div>
    </section>

    {#if ($appStore.projects ?? []).length > 0}
      <section class="panel detail-panel">
        <div class="detail-header">
          <h2>Selected Project Status</h2>
          {#if selectedProject}
            <button
              aria-label={`Refresh status for ${selectedProject.name}`}
              class="icon-refresh"
              on:click={() => appStore.refreshProjectStatus(selectedProject.id)}
              title="Refresh status"
              type="button"
            >
              ↻
            </button>
          {/if}
        </div>

        {#if selectedProject && selectedStatus}
          <p class="muted">Status and process details for the currently selected project.</p>
          <dl class="detail-grid">
            <div>
              <dt>Name</dt>
              <dd>{selectedProject.name}</dd>
            </div>
            <div>
              <dt>Phase</dt>
              <dd>{selectedStatus.phase}</dd>
            </div>
            <div>
              <dt>Project path</dt>
              <dd>{selectedProject.projectPath}</dd>
            </div>
            <div>
              <dt>Assigned port</dt>
              <dd>{selectedProject.assignedPort}</dd>
            </div>
            <div>
              <dt>Service</dt>
              <dd>{selectedStatus.runtimeLabel}</dd>
            </div>
            <div>
              <dt>Resolved JAR</dt>
              <dd>{selectedStatus.resolvedJarPath || "Not resolved yet"}</dd>
            </div>
            <div>
              <dt>Workspace</dt>
              <dd>{selectedStatus.workspaceDir}</dd>
            </div>
            <div>
              <dt>Log file</dt>
              <dd>{selectedStatus.logPath || "Will be created on first launch"}</dd>
            </div>
            <div>
              <dt>PID</dt>
              <dd>{selectedStatus.pid ?? "Not running"}</dd>
            </div>
            <div>
              <dt>Service mode</dt>
              <dd>{selectedStatus.serviceMode}</dd>
            </div>
            <div>
              <dt>Health detail</dt>
              <dd>{selectedStatus.detail}</dd>
            </div>
          </dl>
        {:else}
          <p class="muted">Choose a project to inspect runtime and health details.</p>
        {/if}
      </section>
    {/if}
  {:else if currentView === 'settings'}
    <section class="layout">
      <div class="stack">
        <RuntimeSettings
          disabled={$appStore.isBusy}
          installedRuntime={$appStore.installedRuntime}
          releaseStatus={$appStore.releaseStatus}
          settings={$appStore.settings}
          on:download={() => appStore.downloadLatestRuntime()}
          on:refresh={() => appStore.load()}
          on:save={handleSettingsSave}
        />
      </div>

      <div class="panel stack">
        <details class="advanced-toggle">
          <summary>Diagnostics &amp; paths</summary>
          <div class="stack advanced-content">
            <p class="muted">Diagnostic paths and manager configuration locations.</p>
            {#if $appStore.bootstrap}
              <div class="bootstrap-grid">
                <div>
                  <span class="label">Projects</span>
                  <strong>{$appStore.bootstrap.projectsFile}</strong>
                </div>
                <div>
                  <span class="label">Settings</span>
                  <strong>{$appStore.bootstrap.settingsFile}</strong>
                </div>
                <div>
                  <span class="label">Data root</span>
                  <strong>{$appStore.bootstrap.defaultDataRoot}</strong>
                </div>
                <div>
                  <span class="label">Health</span>
                  <strong>{$appStore.bootstrap.healthStrategy}</strong>
                </div>
              </div>
            {/if}
          </div>
        </details>
      </div>
    </section>
  {/if}
</main>
