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

export interface TraceItem {
  id: string;
  at: number;
  kind: string;
  role: string;
  lot?: string | null;
  tool?: string | null;
  path?: string | null;
  command?: string | null;
  status?: string | null;
  summary: string;
  evidence: string[];
}

export interface Decision {
  at: number;
  kind: string;
  id: string;
  action: string;
  reason: string;
  by: string;
}

export interface MemoryItem {
  id: number;
  title: string;
  status: string;
  kind: string;
  confidence: number;
  evidence: string[];
}

export interface MemoryCandidate extends MemoryItem {
  project: string;
  run: string;
  body: string;
  applicability: string;
  tags: string[];
  source: string;
  created_at: number;
  reviewed_at?: number | null;
  review_reason?: string | null;
}

export interface UsageView {
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_write_tokens: number;
  cost_usd: number;
  requests: number;
}

/** `harnes.view.v1`: typed cards, never model-generated HTML. */
export interface RunView {
  schema: "harnes.view.v1";
  run: string;
  task: string;
  phase: string;
  started_at: number;
  plan: { lots: { id: string; title: string; instructions: string; files?: string[] }[] } | null;
  reviews: Review[];
  gate: Gate | null;
  metrics: { mode: string; eco: boolean; rounds: number; passed: boolean; important: number; nits: number; pre_existing: number; resolved: Record<string, number> } | null;
  trace: TraceItem[];
  findings: { rule: string; severity: string; message: string; evidence: string[] }[];
  usage: UsageView;
  decisions: Decision[];
  memory: MemoryItem[];
  files: string[];
}

export interface HarnessSpec {
  version: number;
  role: string;
  provider: string;
  components: {
    system_prompt: string;
    guardrails: string;
    memory: string;
    tool_guidance: string;
    subagents: string[];
    skills: string[];
    scripts: string[];
    plugins: string[];
  };
  budgets: { always_on_tokens: number; on_demand_tokens: number };
  provenance: string;
}

export interface EvalCheck {
  name: string;
  command: string;
  exit: number | null;
  passed: boolean;
  timed_out: boolean;
  duration_ms: number;
  stdout_tail: string;
  stderr_tail: string;
}

export interface EvalResult {
  schema: "harnes.eval.v1";
  suite: string;
  started_at: number;
  duration_ms: number;
  passed: boolean;
  passed_tasks: number;
  total_tasks: number;
  tasks: { id: string; title: string; passed: boolean; checks: EvalCheck[] }[];
}

export interface EvalSuite {
  name: string;
  description: string;
  tasks: { id: string; title: string; prompt: string; checks: unknown[]; timeout_secs: number }[];
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
  /** Every run of the project, summed (review rounds and findings). */
  runs?: { runs: number; passed: number; first_pass: number; mean_rounds: number; important: number; nits: number; pre_existing: number; resolved: Record<string, number> };
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

export interface PermissionAsk {
  id: string;
  agent: string;
  tool: string;
  summary: string;
  input: unknown;
  can_remember: boolean;
  at: number;
}

export interface ScreenPrompt {
  pane: string;
  agent: string;
  project: string;
  prompt: { question: string; choices: { keys: string; label: string }[] } | null;
}

export interface ConvItem {
  role: "user" | "assistant" | "tool";
  text: string;
  tool?: string;
  at?: string;
}

export interface Conversation {
  started: boolean;
  state: AgentState | null;
  pane: string | null;
  items: ConvItem[];
}

export interface OsPack {
  id: string;
  title: string;
  version: string;
  purpose: string;
  provides: string[];
  depends_on: string[];
  confidence: string;
  skills: string[];
  agents: string[];
  roles: string[];
}
export interface OsDetail {
  pack: OsPack;
  os: string;
  oracle: string;
}

/** The account's limits as Anthropic reports them (0..1 used, resets in epoch seconds). */
export interface Budget {
  limits: { five_hour: number; five_hour_reset: number; seven_day: number; seven_day_reset: number; status: string; seen: number };
  week_elapsed: number;
  eco: boolean;
}

export const api = {
  budgetGet: () => invoke<Budget | null>("budget_get"),
  osList: () => invoke<{ dir: string; packs: OsPack[] }>("os_list"),
  osPack: (id: string) => invoke<OsDetail>("os_pack", { id }),
  osRead: (id: string, path: string) => invoke<string>("os_read", { id, path }),
  osImport: (path: string) => invoke<string[]>("os_import", { path }),
  permissions: () => invoke<PermissionAsk[]>("permissions_pending"),
  answerPermission: (id: string, decision: "allow" | "always" | "deny") => invoke<void>("permission_answer", { id, decision }),
  screenPrompts: () => invoke<ScreenPrompt[]>("screen_prompts"),
  answerScreen: (pane: string, keys: string) => invoke<void>("screen_answer", { pane, keys }),
  conversation: (project?: string) => invoke<Conversation>("conversation", { project }),
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

  // Typed run view, trace, review-gated memory, eval and harness policy.
  runView: (run?: string) => invoke<RunView>("run_view", { run }),
  traceEvents: (params: { run?: string; kind?: string; role?: string; path?: string; limit?: number } = {}) =>
    invoke<TraceItem[]>("trace_events", params),
  traceRuns: () => invoke<[string, number, number][]>("trace_runs"),
  traceSummary: (run: string) => invoke<Record<string, unknown>>("trace_summary", { run }),
  runDecide: (run: string, kind: string, id: string, action: string, reason: string) =>
    invoke<unknown>("run_decide", { run, kind, id, action, reason }),
  memoryCandidates: (status = "candidate", run?: string, limit = 50) =>
    invoke<MemoryCandidate[]>("memory_candidates", { status, run, limit }),
  memoryRecords: (limit = 100) => invoke<MemoryItem[]>("memory_records", { limit }),
  memoryStats: () => invoke<{ candidates: number; pending: number; approved: number; rejected: number; records: number }>("memory_stats"),
  memoryReview: (id: number, decision: "approve" | "reject", reason?: string, title?: string, body?: string) =>
    invoke<MemoryCandidate>("memory_review", { id, decision, reason, title, body }),
  memoryPropose: (input: { title: string; body: string; kind?: string; applicability?: string; tags?: string[]; evidence?: string[]; confidence?: number }) =>
    invoke<MemoryCandidate>("memory_propose", input),
  harnessGet: () => invoke<{ spec: HarnessSpec; components: Record<string, unknown>; path: string }>("harness_get"),
  harnessSave: (spec: HarnessSpec) => invoke<{ ok: boolean; spec: HarnessSpec }>("harness_save", { spec }),
  guardsGet: () => invoke<unknown>("guards_get"),
  guardsSave: (guards: unknown) => invoke<{ ok: boolean }>("guards_save", { guards }),
  guardsReload: () => invoke<{ ok: boolean }>("guards_reload"),
  evalList: () => invoke<EvalSuite[]>("eval_list"),
  evalRun: (suite?: string) => invoke<{ result: EvalResult; path: string; ledger: unknown }>("eval_run", { suite }),
  tunePlan: (component = "worker", suite = "smoke", model = "sonnet") =>
    invoke<{ plan: unknown; path: string }>("tune_plan", { component, suite, model }),
  worktreeList: () => invoke<{ lot: string; branch: string; path: string; base: string }[]>("worktree_list"),
  worktreeRemove: (run: string, lot: string, force = false) => invoke<{ ok: boolean }>("worktree_remove", { run, lot, force }),
  handoffs: () => invoke<{ name: string; path: string; modified: number }[]>("handoffs"),
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
