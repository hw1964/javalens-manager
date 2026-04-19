<script lang="ts">
  import { onMount } from "svelte";
  import ProjectForm from "./lib/components/ProjectForm.svelte";
  import ProjectList from "./lib/components/ProjectList.svelte";
  import { createAppStore } from "./lib/stores/app";
  import type { AddProjectInput } from "./lib/api/tauri";

  const appStore = createAppStore();

  $: selectedProject = $appStore.projects.find(
    (project) => project.id === $appStore.selectedProjectId
  );
  $: selectedStatus = selectedProject
    ? $appStore.runtimeStatuses[selectedProject.id]
    : undefined;

  onMount(() => {
    appStore.load();
  });

  function handleProjectSubmit(event: CustomEvent<AddProjectInput>) {
    appStore.addProjectEntry(event.detail);
  }
</script>

<svelte:head>
  <title>javalens-manager</title>
</svelte:head>

<main class="app-shell">
  <header class="hero panel">
    <div>
      <p class="eyebrow">Sprint 1 thin slice</p>
      <h1>javalens-manager</h1>
      <p class="muted">
        Manual Tauri scaffold for one-project registration plus JavaLens start/stop/status.
      </p>
    </div>

    {#if $appStore.bootstrap}
      <div class="bootstrap-grid">
        <div>
          <span class="label">Transport</span>
          <strong>{$appStore.bootstrap.transport}</strong>
        </div>
        <div>
          <span class="label">Health</span>
          <strong>{$appStore.bootstrap.healthStrategy}</strong>
        </div>
        <div>
          <span class="label">Config</span>
          <strong>{$appStore.bootstrap.configFile}</strong>
        </div>
        <div>
          <span class="label">Logs</span>
          <strong>{$appStore.bootstrap.logDir}</strong>
        </div>
      </div>
    {/if}
  </header>

  {#if $appStore.error}
    <div class="banner error">
      <span>{$appStore.error}</span>
      <button on:click={() => appStore.clearError()} type="button">Dismiss</button>
    </div>
  {/if}

  <section class="layout">
    <ProjectForm disabled={$appStore.isBusy} on:submit={handleProjectSubmit} />

    <ProjectList
      disabled={$appStore.isBusy}
      onRefresh={(projectId) => appStore.refreshProjectStatus(projectId)}
      onSelect={(projectId) => appStore.selectProject(projectId)}
      onStart={(projectId) => appStore.startProject(projectId)}
      onStop={(projectId) => appStore.stopProject(projectId)}
      projects={$appStore.projects}
      runtimeStatuses={$appStore.runtimeStatuses}
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
          <dt>JavaLens JAR</dt>
          <dd>{selectedProject.javalensJarPath}</dd>
        </div>
        <div>
          <dt>Workspace</dt>
          <dd>{selectedStatus.workspaceDir}</dd>
        </div>
        <div>
          <dt>Log file</dt>
          <dd>{selectedStatus.logPath}</dd>
        </div>
        <div>
          <dt>PID</dt>
          <dd>{selectedStatus.pid ?? "Not running"}</dd>
        </div>
        <div>
          <dt>Health detail</dt>
          <dd>{selectedStatus.detail}</dd>
        </div>
      </dl>
    {:else}
      <div class="empty-state">
        Choose a project to inspect its runtime state.
      </div>
    {/if}
  </section>
</main>
