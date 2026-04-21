import { invoke } from "@tauri-apps/api/core";

export type RuntimePhase = "stopped" | "starting" | "running" | "failed";
export type UpdatePolicy = "always" | "ask";
export type McpMergeMode = "safeMerge" | "replaceManagedSection";
export type DeployMode = "deploy" | "dryRun" | "preview" | "regenerate" | "delete";
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
  useSystemTray: boolean;
  mcpClientPaths: McpClientPaths;
  mcpMergeMode: McpMergeMode;
  mcpBackupBeforeWrite: boolean;
  deployTargets: DeployTargetFlags;
  lastReleaseCheck?: string | null;
  lastSeenLatestVersion?: string | null;
}

export interface McpClientPathEntry {
  autoDetectedPath?: string | null;
  manualOverridePath?: string | null;
  effectivePath?: string | null;
}

export interface McpClientPaths {
  cursor: McpClientPathEntry;
  claude: McpClientPathEntry;
  antigravity: McpClientPathEntry;
  intellij: McpClientPathEntry;
}

export interface DeployTargetFlags {
  cursor: boolean;
  claude: boolean;
  antigravity: boolean;
  intellij: boolean;
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
  useSystemTray: boolean;
  mcpClientPaths: McpClientPaths;
  mcpMergeMode: McpMergeMode;
  mcpBackupBeforeWrite: boolean;
  deployTargets: DeployTargetFlags;
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
  servicesInventory: ServicesInventory;
}

export interface ServicesInventory {
  available: boolean;
  services: string[];
  detail: string;
}

export interface CleanupSummary {
  target: string;
  deletedFiles: number;
  deletedDirs: number;
  failedPaths: string[];
  detail: string;
}

export interface ServiceProbeResult {
  ok: boolean;
  services: ProbeServiceEntry[];
  detail: string;
  durationMs: number;
  rawProtocolError?: string | null;
}

export interface ProbeServiceEntry {
  name: string;
  description?: string | null;
}

export type DeployClientStatus = "success" | "skipped" | "failed";

export interface DeployClientResult {
  client: string;
  targetPath: string;
  status: DeployClientStatus;
  message: string;
  backupPath?: string | null;
  changedSections: string[];
  validationErrors: string[];
  previewContent?: string | null;
}

export interface DeployToAgentsInput {
  mode: DeployMode;
  targetClients?: string[] | null;
}

export interface DeployToAgentsResult {
  mode: DeployMode;
  ok: boolean;
  detail: string;
  durationMs: number;
  clients: DeployClientResult[];
}

export interface QuitPromptContext {
  runningServices: number;
  trayEnabled: boolean;
}

export type QuitAction = "cancel" | "hideToTray" | "stopAndQuit" | "quit";

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

export function startAllRuntimes(): Promise<ManagerDashboard> {
  return invoke("start_all_runtimes");
}

export function stopAllRuntimes(): Promise<ManagerDashboard> {
  return invoke("stop_all_runtimes");
}

export function deleteAllProjects(): Promise<ManagerDashboard> {
  return invoke("delete_all_projects");
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

export function redetectMcpClientPaths(): Promise<ManagerDashboard> {
  return invoke("redetect_mcp_client_paths");
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

export function getServicesInventory(): Promise<ServicesInventory> {
  return invoke("get_services_inventory");
}

export function cleanLogs(): Promise<CleanupSummary> {
  return invoke("clean_logs");
}

export function cleanWorkspaces(): Promise<CleanupSummary> {
  return invoke("clean_workspaces");
}

export function cleanGeneratedData(): Promise<CleanupSummary> {
  return invoke("clean_generated_data");
}

export function probeServices(): Promise<ServiceProbeResult> {
  return invoke("probe_services");
}

export function deployToAgents(input: DeployToAgentsInput): Promise<DeployToAgentsResult> {
  return invoke("deploy_to_agents", { input });
}

export function getQuitPromptContext(): Promise<QuitPromptContext> {
  return invoke("get_quit_prompt_context");
}

export function performQuitAction(action: QuitAction): Promise<void> {
  return invoke("perform_quit_action", { action });
}
