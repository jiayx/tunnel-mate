import { invoke, Channel } from "@tauri-apps/api/core";
import { listen, Event } from "@tauri-apps/api/event";

// TypeScript Interfaces matching Rust Models
// Type Definitions

export type ForwardKind = "local" | "remote" | "socks5";

export type TunnelStatus = "stopped" | "running" | "connecting" | "reconnecting" | "failed";

export interface Endpoint {
  host: string;
  port: number;
}

export type ForwardSpec =
  | { kind: "local"; listen: Endpoint; target: Endpoint }
  | { kind: "remote"; listen: Endpoint; target: Endpoint }
  | { kind: "socks5"; listen: Endpoint };

export interface Group {
  id: string;
  name: string;
  description?: string;
}

export interface Tunnel {
  id: string;
  name: string;
  description?: string;
  groupId?: string;

  // SSH connection settings
  sshHost: string;
  sshPort: number;
  sshUser: string;
  sshIdentityFile?: string;
  sshPassword?: string;

  // Jump Host
  jumpHostEnabled: boolean;
  jumpHost?: string;
  jumpPort?: number;
  jumpUser?: string;
  jumpIdentityFile?: string;
  jumpPassword?: string;

  // Forwarding
  forward: ForwardSpec;

  // Behaviors
  startWithApp: boolean;
  autoReconnect: boolean;
  retryCount: number;
  retryInterval: number;
}

export interface GlobalSettings {
  launchOnStartup: boolean;
  startMinimized: boolean;
  closeToTray: boolean;
  keepAliveInterval: number;
  connectTimeout: number;
  sshConfigPath?: string;
}

export interface AppConfig {
  version: number;
  groups: Group[];
  tunnels: Tunnel[];
  settings: GlobalSettings;
}

export function getListenEndpoint(tunnel: Tunnel): Endpoint {
  return tunnel.forward.listen;
}

export function getTargetEndpoint(tunnel: Tunnel): Endpoint | undefined {
  return tunnel.forward.kind === "socks5" ? undefined : tunnel.forward.target;
}

export function formatEndpoint(endpoint: Endpoint): string {
  return `${endpoint.host}:${endpoint.port}`;
}

// SSH Config Interfaces

export interface SshHostConfig {
  host: string;
  hostName?: string;
  user?: string;
  port?: number;
  identityFile?: string;
}

// Diagnostic Interfaces

export interface DiagnosticStep {
  name: string;
  status: "success" | "warning" | "error";
  message: string;
}

// Log Event Interfaces

export interface LogEvent {
  id: string;
  sessionId?: string;
  timestamp: string;
  tunnelId?: string;
  tunnelName?: string;
  eventType: "created" | "updated" | "connecting" | "started" | "stopped" | "restarted" | "reconnected" | "failed" | "deleted";
  message: string;
}

export interface StatusPayload {
  tunnelId: string;
  status: TunnelStatus;
  message?: string;
}

export interface LogPayload {
  tunnelId: string;
  log: string;
}

// Tauri Command Invokers
export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_config");
}

export async function saveConfig(config: AppConfig): Promise<void> {
  return invoke<void>("save_config", { config });
}

export async function getEvents(): Promise<LogEvent[]> {
  return invoke<LogEvent[]>("get_events");
}

export async function importSshConfig(): Promise<SshHostConfig[]> {
  return invoke<SshHostConfig[]>("import_ssh_config");
}

export async function selectPrivateKeyFile(): Promise<string | null> {
  return invoke<string | null>("select_private_key_file");
}

export async function testConnection(tunnel: Tunnel, passphrase?: string): Promise<DiagnosticStep[]> {
  return invoke<DiagnosticStep[]>("test_connection", { tunnel, passphrase: passphrase || null });
}

export async function startTunnel(tunnelId: string, logChannel: Channel<string>, passphrase?: string): Promise<void> {
  return invoke<void>("start_tunnel", { tunnelId, passphrase: passphrase || null, logChannel });
}

export async function stopTunnel(tunnelId: string): Promise<void> {
  return invoke<void>("stop_tunnel", { tunnelId });
}

export async function getTunnelStatus(tunnelId: string): Promise<TunnelStatus> {
  return invoke<TunnelStatus>("get_tunnel_status", { tunnelId });
}

export async function exportConfig(): Promise<string> {
  return invoke<string>("export_config");
}

export async function importConfig(configStr: string): Promise<void> {
  return invoke<void>("import_config", { configStr });
}

export async function clearEvents(): Promise<void> {
  return invoke<void>("clear_events");
}

// Tauri Event Listeners
export function listenToStatusChanges(callback: (payload: StatusPayload) => void) {
  return listen<StatusPayload>("tunnel-status-changed", (event: Event<StatusPayload>) => {
    callback(event.payload);
  });
}

export function listenToActivityEvents(callback: (event: LogEvent) => void) {
  return listen<LogEvent>("activity-event-created", (event: Event<LogEvent>) => {
    callback(event.payload);
  });
}

export function listenToTrayToggle(callback: (tunnelId: string) => void) {
  return listen<string>("tray-toggle-tunnel", (event: Event<string>) => {
    callback(event.payload);
  });
}

export function listenToLogs(callback: (payload: LogPayload) => void) {
  return listen<LogPayload>("tunnel-log", (event: Event<LogPayload>) => {
    callback(event.payload);
  });
}
