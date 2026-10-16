import { Channel, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type AgentState = "starting" | "idle" | "working" | "blocked" | "done" | "exited";
export type Phase = "idle" | "planning" | "calibrating" | "working" | "reviewing" | "gating" | "fixing" | "done" | "failed" | "cancelled";

export interface PaneInfo {
  id: string;
  title: string;
  program: string;
  cwd: string;
  created: number;
  pid?: number | null;
  project?: string | null;
}

export interface ProjectSummary {
  id: string;
  name: string;
  root: string;
  active: boolean;
  running: boolean;
  phase?: Phase | null;
  panes: number;
  working: number;
  waiting: number;
  proxy_port: number;
}

export interface AppInfo {
  project: string | null;
  project_id: string | null;
  proxy_port: number | null;
  hook_port: number;
  ctx_bin: string;
  ctx_bin_found: boolean;
  crew_running: boolean;
}

export interface Commit {
  lot: string;
  message: string;
  files: string[];
}

export interface Review {
  verdict: string;
  lots: Record<string, string>;
  fixes: { lot: string; where: string; problem: string; fix: string }[];
  commits: Commit[];
}

export interface Gate {
  ready: boolean;
  met: string[];
  unmet: { criterion: string; lot: string; problem: string; fix: string }[];
  summary_for_user: string;
}

export interface CrewEvent {
  run: string;
  phase: Phase;
  round: number;
  message: string;
  review?: Review;
  gate?: Gate;
}

export interface Stats {
  proxy: {
    requests: number;
    errors: number;
    bytes_saved: number;
    est_tokens_saved: number;
    elided_blocks: number;
    input_tokens: number;
    output_tokens: number;
    cache_read_tokens: number;
    cache_write_tokens: number;
  } | null;
  ctx: { blobs: number; bytes_stored: number; bytes_returned: number } | null;
}

export type LookStatus = "needs-you" | "working" | "idle" | "finished" | "failed" | "unknown";

export interface LookSession {
  id: string;
  source: string;
  provider: string;
  name: string;
  cwd: string;
  project: string;
  status: LookStatus;
  waiting_reason?: string;
  waiting_text?: string;
  started_at: number;
  status_since: number;
  tokens?: { input: number; cached: number; output: number };
  pid?: number;
  harnes_key?: string;
}

export interface LookEvent {
  at: number;
  session_id: string;
  session_name: string;
  kind: "appeared" | "status-changed" | "ended";
  from?: LookStatus;
  to?: LookStatus;
}

export interface Lookout {
  snapshot: { generated_at: number; sessions: LookSession[]; sources: { source: string; state: string; sessions: number; error?: string }[] };
  events: LookEvent[];
  history: { at: number; needs_you: number; working: number; idle: number; total: number }[];
}

export const api = {
  lookout: () => invoke<Lookout>("lookout"),
  info: () => invoke<AppInfo>("app_info"),
  openProject: (path: string, force = false) =>
    invoke<{ id: string; written: string[]; kept: string[] }>("open_project", { path, force }),
  projects: () => invoke<ProjectSummary[]>("projects_list"),
  activate: (id: string) => invoke<void>("project_activate", { id }),
  closeProject: (id: string) => invoke<void>("project_close", { id }),
  spawn: (kind: "shell" | "claude", project?: string, cols = 120, rows = 32) => invoke<PaneInfo>("pane_spawn", { kind, cols, rows, project }),
  write: (id: string, data: string) => invoke<void>("pane_write", { id, data }),
  resize: (id: string, cols: number, rows: number, cellW: number, cellH: number) =>
    invoke<void>("pane_resize", { id, cols, rows, cellW, cellH }),
  scroll: (id: string, delta: number) => invoke<void>("pane_scroll", { id, delta }),
  close: (id: string) => invoke<void>("pane_close", { id }),
  list: () => invoke<(PaneInfo & { key?: string; role?: string; provider?: string; state?: AgentState })[]>("pane_list"),
  refresh: (id: string) => invoke<void>("pane_refresh", { id }),
  crewStart: (task: string) => invoke<void>("crew_start", { task }),
  say: (text: string) => invoke<string>("orchestrator_say", { text }),
  providers: () => invoke<string[]>("providers_list"),
  switchProvider: (id: string, provider: string) => invoke<string>("pane_switch_provider", { id, provider }),
  crewCancel: () => invoke<void>("crew_cancel"),
  stats: () => invoke<Stats>("stats"),
};

/** Subscribes to binary frames: (paneId, frameBytes). */
export function subscribeFrames(onFrame: (id: string, frame: DataView) => void) {
  const ch = new Channel<ArrayBuffer>();
  const dec = new TextDecoder();
  ch.onmessage = (buf) => {
    const bytes = new Uint8Array(buf);
    const id = dec.decode(bytes.subarray(0, 8)).trim();
    onFrame(id, new DataView(buf, 8));
  };
  return invoke<void>("frames_subscribe", { channel: ch });
}

export function on<T>(event: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(event, (e) => cb(e.payload));
}
