<script lang="ts">
  import { onMount } from "svelte";
  import ProjectForm from "./lib/components/ProjectForm.svelte";
  import ProjectList from "./lib/components/ProjectList.svelte";
  import RuntimeSettings from "./lib/components/RuntimeSettings.svelte";
  import { createAppStore } from "./lib/stores/app";
  import {
    describeRuntimeSource,
    type AddProjectInput,
    type UpdateSettingsInput
  } from "./lib/api/tauri";

  const appStore = createAppStore();

  let currentView: "dashboard" | "settings" = "dashboard";

  $: selectedProject = $appStore.projects?.find((project) => project.id === $appStore.selectedProjectId);
  $: selectedStatus = selectedProject
    ? $appStore.runtimeStatuses?.[selectedProject.id]
    : undefined;

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
        <p class="eyebrow">Sprint 3 UI Cleanup</p>
        <h1>javalens-manager</h1>
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
    <section class="layout">
      <div class="stack">
        <ProjectForm
          defaultManagedRuntimeVersion={$appStore.settings?.defaultManagedRuntimeVersion}
          disabled={$appStore.isBusy}
          installedRuntimes={$appStore.installedRuntimes ?? []}
          on:submit={handleProjectSubmit}
        />
      </div>

      <ProjectList
        disabled={$appStore.isBusy}
        onRefresh={(projectId) => appStore.refreshProjectStatus(projectId)}
        onSelect={(projectId) => appStore.selectProject(projectId)}
        onStart={(projectId) => appStore.startProject(projectId)}
        onStop={(projectId) => appStore.stopProject(projectId)}
        projects={$appStore.projects ?? []}
        runtimeStatuses={$appStore.runtimeStatuses ?? {}}
        selectedProjectId={$appStore.selectedProjectId}
      />
    </section>

    <section class="panel detail-panel">
      <div class="detail-header">
        <h2>Selected Runtime</h2>
        {#if selectedProject}
          <button on:click={() => appStore.refreshProjectStatus(selectedProject.id)} type="button">
            Refresh status
          </button>
        {/if}
      </div>

      {#if selectedProject && selectedStatus}
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
            <dt>Runtime source</dt>
            <dd>{describeRuntimeSource(selectedProject.runtimeSource)}</dd>
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
        <div class="empty-state">
          Choose a project to inspect its runtime selection and current status.
        </div>
      {/if}
    </section>
  {:else if currentView === 'settings'}
    <section class="layout">
      <div class="stack">
        <RuntimeSettings
          disabled={$appStore.isBusy}
          installedRuntimes={$appStore.installedRuntimes ?? []}
          releaseStatus={$appStore.releaseStatus}
          settings={$appStore.settings}
          on:download={() => appStore.downloadLatestRuntime()}
          on:refresh={() => appStore.load()}
          on:save={handleSettingsSave}
        />
      </div>

      <div class="panel stack">
        <div>
          <h2>System Information</h2>
          <p class="muted">Diagnostic paths and manager configuration locations.</p>
        </div>
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
              <span class="label">Managed tools</span>
              <strong>{$appStore.bootstrap.toolsDir}</strong>
            </div>
            <div>
              <span class="label">Health</span>
              <strong>{$appStore.bootstrap.healthStrategy}</strong>
            </div>
          </div>
        {/if}
      </div>
    </section>
  {/if}
</main>
