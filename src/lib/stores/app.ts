import { writable } from "svelte/store";
import {
  addProject,
  getBootstrapStatus,
  getRuntimeStatus,
  listProjects,
  startRuntime,
  stopRuntime,
  type AddProjectInput,
  type BootstrapStatus,
  type ProjectRecord,
  type RuntimeStatusRecord
} from "../api/tauri";

interface AppState {
  bootstrap?: BootstrapStatus;
  projects: ProjectRecord[];
  runtimeStatuses: Record<string, RuntimeStatusRecord>;
  selectedProjectId?: string;
  isBusy: boolean;
  error?: string;
}

const initialState: AppState = {
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

  async function load() {
    update((state) => ({ ...state, isBusy: true, error: undefined }));

    try {
      const [bootstrap, projects] = await Promise.all([
        getBootstrapStatus(),
        listProjects()
      ]);

      const statuses = Object.fromEntries(
        await Promise.all(
          projects.map(async (project) => [project.id, await getRuntimeStatus(project.id)] as const)
        )
      );

      update((state) => ({
        ...state,
        bootstrap,
        projects,
        runtimeStatuses: statuses,
        selectedProjectId: state.selectedProjectId ?? projects[0]?.id,
        isBusy: false
      }));
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
      const created = await addProject(input);
      const status = await getRuntimeStatus(created.id);

      update((state) => ({
        ...state,
        projects: [...state.projects, created],
        runtimeStatuses: {
          ...state.runtimeStatuses,
          [created.id]: status
        },
        selectedProjectId: created.id,
        isBusy: false
      }));
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
      update((state) => ({
        ...state,
        runtimeStatuses: {
          ...state.runtimeStatuses,
          [projectId]: status
        },
        isBusy: false
      }));
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
      update((state) => ({
        ...state,
        runtimeStatuses: {
          ...state.runtimeStatuses,
          [projectId]: status
        },
        isBusy: false
      }));
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
          ...state.runtimeStatuses,
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
    startProject,
    stopProject,
    refreshProjectStatus,
    selectProject,
    clearError
  };
}
