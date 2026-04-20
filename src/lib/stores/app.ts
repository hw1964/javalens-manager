import { writable } from "svelte/store";
import {
  addProject,
  deleteAllProjects,
  deleteProject,
  downloadOrUpdateJavalens,
  getDashboard,
  getRuntimeStatus,
  startAllRuntimes,
  startRuntime,
  stopAllRuntimes,
  stopRuntime,
  updateProjectPort as updateProjectPortApi,
  updateSettings,
  type AddProjectInput,
  type ManagerDashboard,
  type RuntimeStatusRecord,
  type UpdateSettingsInput
} from "../api/tauri";

interface AppState extends Partial<ManagerDashboard> {
  selectedProjectId?: string;
  isBusy: boolean;
  error?: string;
  projectErrors?: Record<string, string>;
}

const initialState: AppState = {
  projects: [],
  runtimeStatuses: {},
  projectErrors: {},
  isBusy: false
};

function normalizeError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

export function createAppStore() {
  const { subscribe, update } = writable<AppState>(initialState);
  const STATUS_POLL_INTERVAL_MS = 2500;
  let pollTimer: ReturnType<typeof setInterval> | undefined;
  let pollInFlight = false;
  let visibilityHandlerAttached = false;

  function syncDashboard(dashboard: ManagerDashboard) {
    update((state) => ({
      ...state,
      ...dashboard,
      projectErrors: Object.fromEntries(
        Object.entries(state.projectErrors ?? {}).filter(([projectId]) =>
          dashboard.projects.some((project) => project.id === projectId)
        )
      ),
      selectedProjectId:
        state.selectedProjectId && dashboard.projects.some((project) => project.id === state.selectedProjectId)
          ? state.selectedProjectId
          : dashboard.projects[0]?.id,
      isBusy: false,
      error: undefined
    }));
  }

  async function load() {
    update((state) => ({ ...state, isBusy: true, error: undefined }));

    try {
      syncDashboard(await getDashboard());
      ensureStatusPolling();
    } catch (error) {
      update((state) => ({
        ...state,
        isBusy: false,
        error: normalizeError(error)
      }));
    }
  }

  async function addProjectEntry(input: AddProjectInput) {
    update((state) => ({ ...state, isBusy: true, error: undefined }));

    try {
      await addProject(input);
      syncDashboard(await getDashboard());
    } catch (error) {
      update((state) => ({
        ...state,
        isBusy: false,
        error: normalizeError(error)
      }));
    }
  }

  async function updateProjectPortEntry(projectId: string, assignedPort: number) {
    update((state) => ({ ...state, isBusy: true, error: undefined }));
    try {
      syncDashboard(await updateProjectPortApi({ projectId, assignedPort }));
      clearProjectError(projectId);
    } catch (error) {
      setProjectError(projectId, error);
    }
  }

  async function updateManagerSettings(input: UpdateSettingsInput) {
    update((state) => ({ ...state, isBusy: true, error: undefined }));

    try {
      syncDashboard(await updateSettings(input));
    } catch (error) {
      update((state) => ({
        ...state,
        isBusy: false,
        error: normalizeError(error)
      }));
    }
  }

  async function deleteProjectEntry(projectId: string) {
    update((state) => ({ ...state, isBusy: true, error: undefined }));
    try {
      syncDashboard(await deleteProject(projectId));
    } catch (error) {
      update((state) => ({
        ...state,
        isBusy: false,
        error: normalizeError(error)
      }));
    }
  }

  async function deleteAllProjectEntries() {
    update((state) => ({ ...state, isBusy: true, error: undefined }));
    try {
      syncDashboard(await deleteAllProjects());
    } catch (error) {
      update((state) => ({
        ...state,
        isBusy: false,
        error: normalizeError(error)
      }));
    }
  }

  async function downloadLatestRuntime() {
    update((state) => ({ ...state, isBusy: true, error: undefined }));

    try {
      syncDashboard(await downloadOrUpdateJavalens());
    } catch (error) {
      update((state) => ({
        ...state,
        isBusy: false,
        error: normalizeError(error)
      }));
    }
  }

  async function startProject(projectId: string) {
    update((state) => ({ ...state, isBusy: true, error: undefined }));

    try {
      const status = await startRuntime(projectId);
      mergeRuntimeStatus(projectId, status);
    } catch (error) {
      setProjectError(projectId, error);
    }
  }

  async function startAllProjects() {
    update((state) => ({ ...state, isBusy: true, error: undefined }));
    try {
      syncDashboard(await startAllRuntimes());
    } catch (error) {
      update((state) => ({
        ...state,
        isBusy: false,
        error: normalizeError(error)
      }));
    }
  }

  async function stopAllProjects() {
    update((state) => ({ ...state, isBusy: true, error: undefined }));
    try {
      syncDashboard(await stopAllRuntimes());
    } catch (error) {
      update((state) => ({
        ...state,
        isBusy: false,
        error: normalizeError(error)
      }));
    }
  }

  async function stopProject(projectId: string) {
    update((state) => ({ ...state, isBusy: true, error: undefined }));

    try {
      const status = await stopRuntime(projectId);
      mergeRuntimeStatus(projectId, status);
      clearProjectError(projectId);
    } catch (error) {
      update((state) => ({
        ...state,
        isBusy: false,
        error: normalizeError(error)
      }));
    }
  }

  async function refreshProjectStatus(projectId: string) {
    try {
      const status = await getRuntimeStatus(projectId);
      update((state) => ({
        ...state,
        runtimeStatuses: {
          ...(state.runtimeStatuses ?? {}),
          [projectId]: status
        }
      }));
      clearProjectError(projectId);
    } catch (error) {
      update((state) => ({
        ...state,
        error: normalizeError(error)
      }));
    }
  }

  async function refreshAllProjectStatuses() {
    if (pollInFlight || typeof document === "undefined" || document.hidden) {
      return;
    }

    let projectIds: string[] = [];
    update((state) => {
      projectIds = (state.projects ?? []).map((project) => project.id);
      return state;
    });

    if (projectIds.length === 0) {
      return;
    }

    pollInFlight = true;
    try {
      const results = await Promise.all(
        projectIds.map(async (projectId) => ({
          projectId,
          status: await getRuntimeStatus(projectId)
        }))
      );

      update((state) => {
        const runtimeStatuses = { ...(state.runtimeStatuses ?? {}) };
        for (const result of results) {
          runtimeStatuses[result.projectId] = result.status;
        }

        return {
          ...state,
          runtimeStatuses
        };
      });
    } catch (error) {
      update((state) => ({
        ...state,
        error: normalizeError(error)
      }));
    } finally {
      pollInFlight = false;
    }
  }

  function ensureStatusPolling() {
    if (!pollTimer) {
      pollTimer = setInterval(() => {
        void refreshAllProjectStatuses();
      }, STATUS_POLL_INTERVAL_MS);
    }

    if (!visibilityHandlerAttached && typeof document !== "undefined") {
      document.addEventListener("visibilitychange", () => {
        if (!document.hidden) {
          void refreshAllProjectStatuses();
        }
      });
      visibilityHandlerAttached = true;
    }
  }

  function mergeRuntimeStatus(projectId: string, status: RuntimeStatusRecord) {
    update((state) => {
      const projectErrors = { ...(state.projectErrors ?? {}) };
      delete projectErrors[projectId];
      return {
        ...state,
        projectErrors,
        runtimeStatuses: {
          ...(state.runtimeStatuses ?? {}),
          [projectId]: status
        },
        isBusy: false
      };
    });
  }

  function setProjectError(projectId: string, error: unknown) {
    update((state) => ({
      ...state,
      isBusy: false,
      projectErrors: {
        ...(state.projectErrors ?? {}),
        [projectId]: normalizeError(error)
      }
    }));
  }

  function clearProjectError(projectId: string) {
    update((state) => {
      if (!state.projectErrors?.[projectId]) {
        return state;
      }
      const projectErrors = { ...(state.projectErrors ?? {}) };
      delete projectErrors[projectId];
      return {
        ...state,
        projectErrors
      };
    });
  }

  function selectProject(projectId: string) {
    update((state) => ({
      ...state,
      selectedProjectId: projectId
    }));
  }

  function clearError() {
    update((state) => ({
      ...state,
      error: undefined
    }));
  }

  return {
    subscribe,
    load,
    addProjectEntry,
    updateProjectPortEntry,
    deleteProjectEntry,
    deleteAllProjectEntries,
    updateManagerSettings,
    downloadLatestRuntime,
    startProject,
    startAllProjects,
    stopAllProjects,
    stopProject,
    refreshProjectStatus,
    selectProject,
    clearError
  };
}
