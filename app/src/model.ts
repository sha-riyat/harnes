// What the interface reads, derived from the state harnes already keeps (panes,
// permissions, typed questions, the crew's events, the run view, the account's
// limits). Nothing here talks to a model or invents data: a lot with no worker yet
// is "planned", an agent with no hook yet is "idle".

import { createEffect, createMemo, createResource, createRoot, createSignal, onCleanup } from "solid-js";
import { api, on, type Budget, type Conversation, type RunView } from "./api";
import { activeProject, asks, crew, panes, permissions, projectPanes, screenPrompts } from "./store";
import { lotOf, nameOf, roleOf, visual, type Role, type Visual } from "./ui/kit/roles";

export const RUNNING = ["planning", "calibrating", "working", "reviewing", "gating", "fixing"];
export const isRunning = () => RUNNING.includes(crew.phase);

// --- The crew: one entry per agent pane of the active project. ---
export interface Agent {
  id: string;
  key: string;
  name: string;
  role: Role;
  lot?: string;
  state: Visual;
  provider: string;
  paneId: string;
}

export const agents = createMemo<Agent[]>(() =>
  projectPanes()
    .filter((p) => p.key && !p.key.startsWith("claude-"))
    .map((p) => ({
      id: p.id,
      key: p.key!,
      name: nameOf(p.key, p.title),
      role: roleOf(p.key, p.role),
      lot: lotOf(p.key),
      state: visual(p.state, p.exited),
      provider: p.provider ?? "claude",
      paneId: p.id,
    }))
    .sort((a, b) => rank(a.role) - rank(b.role) || a.name.localeCompare(b.name)),
);
const rank = (r: Role) => ["orchestrator", "planner", "teacher", "worker", "scout", "reviewer"].indexOf(r);

// --- What waits for the user, for the active project. ---
const mine = (agentKey: string) => {
  const pane = panes.find((p) => p.key === agentKey);
  return !pane?.project || pane.project === activeProject();
};
export const waitingPermissions = createMemo(() => permissions().filter((a) => mine(a.agent)));
export const waitingAsks = createMemo(() => asks().filter((a) => mine(a.agent)));
export const waitingScreens = createMemo(() => screenPrompts().filter((q) => q.prompt && q.project === activeProject()));
export const needCount = () => waitingPermissions().length + waitingAsks().length + waitingScreens().length;

// --- The run view, shared by the overview and the Run tab. ---
export const [runView, { refetch: refetchRun }] = createRoot(() =>
  createResource(
    () => ({ project: activeProject(), phase: crew.phase, log: crew.log.length }),
    async (src) => (src.project ? await api.runView().catch(() => null) : null) as RunView | null,
  ),
);

export type LotState = "planned" | "working" | "needs" | "review" | "done";
export interface LotRow {
  id: string;
  title: string;
  files: number;
  state: LotState;
  agent?: Agent;
}

/** Lots of the plan, with the state their worker is in. */
export const lots = createMemo<LotRow[]>(() => {
  const plan = runView()?.plan?.lots ?? [];
  const reviews = runView()?.reviews ?? [];
  const reviewed = new Set(reviews.flatMap((r) => Object.keys(r.lots)));
  return plan.map((l) => {
    const agent = agents().find((a) => a.lot === l.id && a.role === "worker");
    let state: LotState = "planned";
    if (crew.phase === "done") state = "done";
    else if (agent?.state === "blocked") state = "needs";
    else if (agent?.state === "working") state = "working";
    else if (agent?.state === "done" || reviewed.has(l.id)) state = reviewed.has(l.id) ? "review" : "done";
    return { id: l.id, title: l.title, files: l.files?.length ?? 0, state, agent };
  });
});

// --- The orchestrator's conversation, shared by the chat and the overview. ---
export const [conv, { refetch: refetchConv }] = createRoot(() => {
  const r = createResource(activeProject, (p) => (p ? api.conversation(p).catch(() => null as Conversation | null) : null));
  on<string>("conversation-changed", (p) => p === activeProject() && r[1].refetch()).then((off) => onCleanup(off)).catch(() => {});
  // The transcript also grows between hook events while the orchestrator works.
  const t = setInterval(() => r[0]()?.state === "working" && r[1].refetch(), 1500);
  onCleanup(() => clearInterval(t));
  return r;
});

// --- The account's limits (read by the proxy on every response). ---
export const [budget, { refetch: refetchBudget }] = createRoot(() => {
  const r = createResource(() => api.budgetGet().catch(() => null as Budget | null));
  const t = setInterval(() => r[1].refetch(), 30_000);
  onCleanup(() => clearInterval(t));
  return r;
});
export const left = (epoch: number) => {
  const s = Math.max(0, epoch - Date.now() / 1000);
  const d = Math.floor(s / 86400), h = Math.floor((s % 86400) / 3600), m = Math.floor((s % 3600) / 60);
  return d ? `${d} d ${h} h` : h ? `${h} h ${String(m).padStart(2, "0")}` : `${m} min`;
};

// --- Notifications: what needs you now, and what the run just did. ---
export interface Notice {
  id: string;
  role: Role;
  who: string;
  text: string;
  at: number;
  ask?: boolean;
  action?: "Answer" | "Review";
}
const roleOfAgent = (key: string) => roleOf(key, panes.find((p) => p.key === key)?.role);

export const notices = createMemo<Notice[]>(() => {
  const out: Notice[] = [];
  for (const a of waitingAsks()) out.push({ id: `ask-${a.id}`, role: roleOfAgent(a.agent), who: nameOf(a.agent), text: `asks: ${a.card.question}`, at: a.at, ask: true, action: "Answer" });
  for (const a of waitingPermissions()) out.push({ id: `perm-${a.id}`, role: roleOfAgent(a.agent), who: nameOf(a.agent), text: `wants to run ${a.summary}`, at: a.at, ask: true, action: "Review" });
  for (const q of waitingScreens()) out.push({ id: `scr-${q.pane}`, role: roleOfAgent(q.agent), who: nameOf(q.agent), text: `asks in its terminal: ${q.prompt!.question}`, at: Date.now(), ask: true, action: "Answer" });
  for (const e of [...crew.log].reverse().slice(0, 12)) {
    if (!e.at || !["done", "failed", "cancelled", "reviewing", "gating"].includes(e.phase)) continue;
    out.push({ id: `crew-${e.at}-${e.phase}`, role: "orchestrator", who: "Orchestrator", text: e.phase === "done" ? "has a summary ready for you" : e.phase === "failed" ? `stopped: ${e.message}` : e.message || e.phase, at: e.at });
  }
  return out.sort((a, b) => Number(!!b.ask) - Number(!!a.ask) || b.at - a.at);
});

const SEEN = "harnes.notices.seen";
const [seen, setSeen] = createSignal(Number((() => { try { return localStorage.getItem(SEEN); } catch { return 0; } })() ?? 0));
export const markNoticesRead = () => {
  const now = Date.now();
  setSeen(now);
  try { localStorage.setItem(SEEN, String(now)); } catch { /* unavailable: kept in memory */ }
};
export const isUnread = (n: Notice) => !!n.ask || n.at > seen();
export const unreadCount = () => notices().filter(isUnread).length;

// Keep the budget fresh when a run changes phase.
createRoot(() => createEffect(() => { crew.phase; refetchBudget(); }));
