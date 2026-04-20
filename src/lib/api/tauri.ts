import { invoke } from "@tauri-apps/api/core";

export type RuntimePhase = "stopped" | "starting" | "running" | "failed";
export type UpdatePolicy = "always" | "ask";
export type ReleaseStatusKind =
  | "ready"
  | "missing"
  | "updateAvailable"
  | "checkFailed"
  | "checkingDisabled";

export interface BootstrapStatus {
  configDir: string;
  stateDir: string;
  cacheDir: string;
  projectsFile: string;
  settingsFile: string;
  runtimeStateFile: string;
  defaultDataRoot: string;
  logDir: string;
  transport: string;
  healthStrategy: string;
}

export interface ManagerSettings {
  version: number;
  updatePolicy: UpdatePolicy;
  autoCheckForUpdates: boolean;
  manualFallbackJarPath?: string | null;
  dataRoot: string;
  globalRuntimeSource: RuntimeSource;
  portRangeStart: number;
  portRangeEnd: number;
  lastReleaseCheck?: string | null;
  lastSeenLatestVersion?: string | null;
}

export type RuntimeSource =
  | {
      kind: "managed";
    }
  | {
      kind: "localJar";
      jarPath: string;
    };

export interface ProjectRecord {
  id: string;
  name: string;
  projectPath: string;
  assignedPort: number;
}

export interface AddProjectInput {
  name: string;
  projectPath: string;
  assignedPort?: number;
}

export interface UpdateSettingsInput {
  updatePolicy: UpdatePolicy;
  autoCheckForUpdates: boolean;
  dataRoot: string;
  globalRuntimeSource: RuntimeSource;
  portRangeStart: number;
  portRangeEnd: number;
}

export interface ManagedRuntimeRecord {
  version: string;
  installDir: string;
  jarPath: string;
  assetName: string;
  installedAt: string;
}

export interface ReleaseStatus {
  kind: ReleaseStatusKind;
  latestVersion?: string | null;
  defaultVersion?: string | null;
  checkedAt?: string | null;
  updateAvailable: boolean;
  detail: string;
}

export interface RuntimeStatusRecord {
  projectId: string;
  phase: RuntimePhase;
  assignedPort: number;
  transport: string;
  pid?: number | null;
  workspaceDir: string;
  logPath: string;
  runtimeLabel: string;
  resolvedJarPath: string;
  serviceMode: string;
  detail: string;
}

export interface ManagerDashboard {
  bootstrap: BootstrapStatus;
  settings: ManagerSettings;
  releaseStatus: ReleaseStatus;
  installedRuntime?: ManagedRuntimeRecord | null;
  projects: ProjectRecord[];
  runtimeStatuses: Record<string, RuntimeStatusRecord>;
  suggestedPort?: number | null;
}

export interface UpdateProjectPortInput {
  projectId: string;
  assignedPort: number;
}

export interface WorkspaceProjectCandidate {
  name: string;
  projectPath: string;
  kind: string;
}

export interface WorkspaceImportInput {
  workspaceFile: string;
  selectedPaths: string[];
}

export interface WorkspaceImportResult {
  added: ProjectRecord[];
  skipped: string[];
}

export function getDashboard(): Promise<ManagerDashboard> {
  return invoke("get_dashboard");
}

export function addProject(input: AddProjectInput): Promise<ProjectRecord> {
  return invoke("add_project", { input });
}

export function suggestNextPort(): Promise<number> {
  return invoke("suggest_next_port");
}

export function updateProjectPort(input: UpdateProjectPortInput): Promise<ManagerDashboard> {
  return invoke("update_project_port", { input });
}

export function deleteProject(projectId: string): Promise<ManagerDashboard> {
  return invoke("delete_project", { projectId });
}

export function discoverWorkspaceProjects(workspaceFile: string): Promise<WorkspaceProjectCandidate[]> {
  return invoke("discover_workspace_projects", { workspaceFile });
}

export function importWorkspaceProjects(input: WorkspaceImportInput): Promise<WorkspaceImportResult> {
  return invoke("import_workspace_projects", { input });
}

export function updateSettings(input: UpdateSettingsInput): Promise<ManagerDashboard> {
  return invoke("update_settings", { input });
}

export function downloadOrUpdateJavalens(): Promise<ManagerDashboard> {
  return invoke("download_or_update_javalens");
}

export function startRuntime(projectId: string): Promise<RuntimeStatusRecord> {
  return invoke("start_runtime", { projectId });
}

export function stopRuntime(projectId: string): Promise<RuntimeStatusRecord> {
  return invoke("stop_runtime", { projectId });
}

export function getRuntimeStatus(projectId: string): Promise<RuntimeStatusRecord> {
  return invoke("get_runtime_status", { projectId });
}
