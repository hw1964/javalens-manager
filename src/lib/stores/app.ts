import { writable } from "svelte/store";
import {
  addProject,
  downloadOrUpdateJavalens,
  getDashboard,
  getRuntimeStatus,
  startRuntime,
  stopRuntime,
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
}

const initialState: AppState = {
  installedRuntimes: [],
  projects: [],
  runtimeStatuses: {},
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

  function syncDashboard(dashboard: ManagerDashboard) {
    update((state) => ({
      ...state,
      ...dashboard,
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
    } catch (error) {
      update((state) => ({
        ...state,
        error: normalizeError(error)
      }));
    }
  }

  function mergeRuntimeStatus(projectId: string, status: RuntimeStatusRecord) {
    update((state) => ({
      ...state,
      runtimeStatuses: {
        ...(state.runtimeStatuses ?? {}),
        [projectId]: status
      },
      isBusy: false
    }));
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
    updateManagerSettings,
    downloadLatestRuntime,
    startProject,
    stopProject,
    refreshProjectStatus,
    selectProject,
    clearError
  };
}
