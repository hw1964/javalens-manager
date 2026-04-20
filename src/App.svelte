<script lang="ts">
  import { onDestroy, onMount } from "svelte";
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
  const MIN_LEFT_PANEL_WIDTH = 260;
  const MAX_LEFT_PANEL_WIDTH = 560;
  const MIN_RIGHT_PANEL_WIDTH = 420;
  const SPLITTER_WIDTH = 12;

  let currentView: "dashboard" | "settings" = "dashboard";
  let leftPanelWidth = 320;
  let isDraggingSplitter = false;
  let isCompactLayout = false;
  let dashboardLayoutEl: HTMLElement | null = null;
  let splitterPointerId: number | null = null;
  let dragStartX = 0;
  let dragStartWidth = 0;
  let lastDashboardWidth = 0;

  $: selectedProject = $appStore.projects?.find((project) => project.id === $appStore.selectedProjectId);
  $: selectedStatus = selectedProject
    ? $appStore.runtimeStatuses?.[selectedProject.id]
    : undefined;
  $: runtimeSource = $appStore.settings?.globalRuntimeSource;
  $: runtimeSubtitle = (() => {
    const prefix = `javalens-manager ${managerBuildVersion} | `;
    if (runtimeSource?.kind === "localJar") {
      const jarPath = runtimeSource.jarPath?.trim() ?? "";
      if (!jarPath) {
        return `${prefix}JavaLens local JAR (path not set)`;
      }
      const jarName = jarPath.split(/[\\/]/).pop() ?? jarPath;
      return `${prefix}JavaLens local JAR ${jarName}`;
    }

    if ($appStore.installedRuntime?.version) {
      return `${prefix}JavaLens ${$appStore.installedRuntime.version}${
        $appStore.releaseStatus?.updateAvailable
          ? ` (update: ${$appStore.releaseStatus.latestVersion ?? "available"})`
          : ""
      }`;
    }
    return `${prefix}JavaLens runtime not downloaded`;
  })();
  $: runtimeSubtitleTitle =
    runtimeSource?.kind === "localJar" && runtimeSource.jarPath?.trim()
      ? runtimeSource.jarPath
      : runtimeSubtitle;

  onMount(() => {
    appStore.load();

    if (typeof window !== "undefined") {
      const mediaQuery = window.matchMedia("(max-width: 960px)");
      const updateCompact = () => {
        const wasCompact = isCompactLayout;
        isCompactLayout = mediaQuery.matches;
        if (wasCompact && !isCompactLayout) {
          lastDashboardWidth = getDashboardWidth();
          applyClampedWidth();
          return;
        }
        lastDashboardWidth = getDashboardWidth();
      };
      updateCompact();
      window.addEventListener("resize", handleWindowResize);
      mediaQuery.addEventListener("change", updateCompact);

      return () => {
        window.removeEventListener("resize", handleWindowResize);
        mediaQuery.removeEventListener("change", updateCompact);
      };
    }
  });

  onDestroy(() => {
    stopSplitterDrag();
  });

  function handleProjectSubmit(event: CustomEvent<AddProjectInput>) {
    appStore.addProjectEntry(event.detail);
  }

  function handleSettingsSave(event: CustomEvent<UpdateSettingsInput>) {
    appStore.updateManagerSettings(event.detail);
  }

  function clampLeftPanelWidth(width: number): number {
    if (isCompactLayout) {
      return leftPanelWidth;
    }

    const layoutWidth = dashboardLayoutEl?.clientWidth ?? window.innerWidth;
    const maxAllowedByLayout = Math.max(
      MIN_LEFT_PANEL_WIDTH,
      layoutWidth - MIN_RIGHT_PANEL_WIDTH - SPLITTER_WIDTH
    );
    const maxWidth = Math.min(MAX_LEFT_PANEL_WIDTH, maxAllowedByLayout);
    return Math.max(MIN_LEFT_PANEL_WIDTH, Math.min(width, maxWidth));
  }

  function getDashboardWidth(): number {
    if (typeof window === "undefined") {
      return 0;
    }
    return dashboardLayoutEl?.clientWidth ?? window.innerWidth;
  }

  function applyClampedWidth() {
    if (isCompactLayout || currentView !== "dashboard") {
      return;
    }
    const clamped = clampLeftPanelWidth(leftPanelWidth);
    if (clamped !== leftPanelWidth) {
      leftPanelWidth = clamped;
    }
  }

  function stopSplitterDrag() {
    if (!isDraggingSplitter) {
      return;
    }
    isDraggingSplitter = false;
    splitterPointerId = null;
    window.removeEventListener("pointermove", handleSplitterPointerMove);
    window.removeEventListener("pointerup", handleSplitterPointerUp);
    window.removeEventListener("pointercancel", handleSplitterPointerUp);
  }

  function handleSplitterPointerMove(event: PointerEvent) {
    if (!isDraggingSplitter) {
      return;
    }
    const delta = event.clientX - dragStartX;
    leftPanelWidth = clampLeftPanelWidth(dragStartWidth + delta);
  }

  function handleSplitterPointerUp(event: PointerEvent) {
    if (splitterPointerId !== null && event.pointerId !== splitterPointerId) {
      return;
    }
    stopSplitterDrag();
    lastDashboardWidth = getDashboardWidth();
  }

  function handleSplitterPointerDown(event: PointerEvent) {
    if (isCompactLayout) {
      return;
    }
    event.preventDefault();
    splitterPointerId = event.pointerId;
    dragStartX = event.clientX;
    dragStartWidth = leftPanelWidth;
    isDraggingSplitter = true;
    leftPanelWidth = clampLeftPanelWidth(leftPanelWidth);
    window.addEventListener("pointermove", handleSplitterPointerMove);
    window.addEventListener("pointerup", handleSplitterPointerUp);
    window.addEventListener("pointercancel", handleSplitterPointerUp);
  }

  function handleWindowResize() {
    if (isCompactLayout || currentView !== "dashboard") {
      lastDashboardWidth = getDashboardWidth();
      return;
    }

    const nextWidth = getDashboardWidth();
    if (nextWidth <= 0) {
      return;
    }

    if (lastDashboardWidth <= 0) {
      lastDashboardWidth = nextWidth;
      applyClampedWidth();
      return;
    }

    const scaledWidth = (leftPanelWidth / lastDashboardWidth) * nextWidth;
    const clamped = clampLeftPanelWidth(scaledWidth);
    if (clamped !== leftPanelWidth) {
      leftPanelWidth = clamped;
    }
    lastDashboardWidth = nextWidth;
  }

  $: if (currentView === "dashboard") {
    applyClampedWidth();
    const width = getDashboardWidth();
    if (width > 0) {
      lastDashboardWidth = width;
    }
  }
</script>

<svelte:head>
  <title>javalens-manager</title>
</svelte:head>

<main class={`app-shell ${currentView === "dashboard" ? "dashboard-shell-mode" : ""}`}>
  <header class="hero panel">
    <div class="header-content">
      <div>
        <h1>javalens-manager</h1>
        <p class="title-subline muted" title={runtimeSubtitleTitle}>{runtimeSubtitle}</p>
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
    <section class="dashboard-main">
      <section class="dashboard-content">
        <section
          bind:this={dashboardLayoutEl}
          class={`layout dashboard-layout ${isDraggingSplitter ? "is-resizing" : ""}`}
          style={`--left-panel-width: ${leftPanelWidth}px;`}
        >
          <div class="dashboard-column">
            <ProjectForm
              disabled={$appStore.isBusy}
              suggestedPort={$appStore.suggestedPort}
              on:submit={handleProjectSubmit}
              on:imported={() => appStore.load()}
            />
          </div>

          <div
            aria-hidden={isCompactLayout}
            class="dashboard-splitter"
            on:pointerdown={handleSplitterPointerDown}
            role="separator"
            tabindex="-1"
          ></div>

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
              onDeploy={(mode) => appStore.deployToAgents(mode)}
              onUpdatePort={(projectId, assignedPort) => appStore.updateProjectPortEntry(projectId, assignedPort)}
              deployBusy={$appStore.deployBusy ?? false}
              deployError={$appStore.deployError}
              lastDeployResult={$appStore.lastDeployResult}
              projects={$appStore.projects ?? []}
              projectErrors={$appStore.projectErrors ?? {}}
              runtimeStatuses={$appStore.runtimeStatuses ?? {}}
              selectedProjectId={$appStore.selectedProjectId}
            />
          </div>
        </section>
      </section>

      {#if ($appStore.projects ?? []).length > 0}
        <section class="panel detail-panel dashboard-footer-panel">
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
            <dl class="detail-grid">
              <div>
                <dt>Name</dt>
                <dd>{selectedProject.name}</dd>
              </div>
              <div>
                <dt>Project path</dt>
                <dd title={selectedProject.projectPath}>{selectedProject.projectPath}</dd>
              </div>
              <div>
                <dt>Assigned port</dt>
                <dd>{selectedProject.assignedPort}</dd>
              </div>
              <div>
                <dt>PID</dt>
                <dd>{selectedStatus.pid ?? "Not running"}</dd>
              </div>
              <div>
                <dt>Phase / Health</dt>
                <dd title={selectedStatus.detail}>
                  {selectedStatus.phase} - {selectedStatus.detail}
                </dd>
              </div>
            </dl>
          {:else}
            <p class="muted">Choose a project to inspect runtime and health details.</p>
          {/if}
        </section>
      {/if}
    </section>
  {:else if currentView === 'settings'}
    <section class="stack">
      <RuntimeSettings
        bootstrap={$appStore.bootstrap}
        disabled={$appStore.isBusy}
        installedRuntime={$appStore.installedRuntime}
        lastCleanupSummary={$appStore.lastCleanupSummary}
        lastServiceProbe={$appStore.lastServiceProbe}
        releaseStatus={$appStore.releaseStatus}
        serviceProbeBusy={$appStore.serviceProbeBusy ?? false}
        serviceProbeError={$appStore.serviceProbeError}
        settings={$appStore.settings}
        on:cleanGeneratedData={() => appStore.cleanAllGeneratedData()}
        on:cleanLogs={() => appStore.cleanAllLogs()}
        on:cleanWorkspaces={() => appStore.cleanAllWorkspaces()}
        on:clearCleanupSummary={() => appStore.clearCleanupSummary()}
        on:clearServiceProbeError={() => appStore.clearServiceProbeError()}
        on:download={() => appStore.downloadLatestRuntime()}
        on:probeServices={() => appStore.probeServices()}
        on:redetectMcpPaths={() => appStore.redetectMcpClientPaths()}
        on:refresh={() => appStore.load()}
        on:save={handleSettingsSave}
      />
    </section>
  {/if}
</main>
