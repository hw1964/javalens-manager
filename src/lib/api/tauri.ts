import { invoke } from "@tauri-apps/api/core";

export type RuntimePhase = "stopped" | "starting" | "running" | "failed";

export interface BootstrapStatus {
  configDir: string;
  stateDir: string;
  cacheDir: string;
  configFile: string;
  workspaceRoot: string;
  logDir: string;
  transport: string;
  healthStrategy: string;
}

export interface ProjectRecord {
  id: string;
  name: string;
  projectPath: string;
  javalensJarPath: string;
  workspaceDir: string;
}

export interface AddProjectInput {
  name: string;
  projectPath: string;
  javalensJarPath: string;
  workspaceDir?: string;
}

export interface RuntimeStatusRecord {
  projectId: string;
  phase: RuntimePhase;
  transport: string;
  pid?: number | null;
  workspaceDir: string;
  logPath: string;
  detail: string;
}

export function getBootstrapStatus(): Promise<BootstrapStatus> {
  return invoke("get_bootstrap_status");
}

export function listProjects(): Promise<ProjectRecord[]> {
  return invoke("list_projects");
}

export function addProject(input: AddProjectInput): Promise<ProjectRecord> {
  return invoke("add_project", { input });
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
