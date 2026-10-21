// Global UI state: several projects, one active; panes and crew runs per project.

import { createMemo, createSignal } from "solid-js";
import { createStore, produce } from "solid-js/store";
import type { AgentState, AppInfo, CrewEvent, Gate, PaneInfo, PermissionAsk, Phase, ProjectSummary, Review, ScreenPrompt, Stats } from "./api";

export interface PaneView extends PaneInfo {
  key?: string;
  role?: string;
  provider?: string;
  state?: AgentState;
  exited?: boolean;
}

export const [panes, setPanes] = createStore<PaneView[]>([]);
export const [projects, setProjects] = createSignal<ProjectSummary[]>([]);
export const [activeProject, setActiveProject] = createSignal<string | null>(null);
export const [info, setInfo] = createSignal<AppInfo | null>(null);
export const [stats, setStats] = createSignal<Stats | null>(null);
export const [focused, setFocused] = createSignal<string | null>(null);
export const [maximized, setMaximized] = createSignal<string | null>(null);
/** Main area: conversation with the orchestrator, terminals grid, or every agent on the machine. */
export const [view, setView] = createSignal<"chat" | "run" | "panes" | "agents" | "os">("chat");
/** Terminals docked under the main view, opened on demand (they stay hidden otherwise). */
export const [dockTabs, setDockTabs] = createSignal<string[]>([]);
/** The docked terminal shown; null folds the dock away. */
export const [drawerPane, showDocked] = createSignal<string | null>(null);
/** Opens a terminal in the dock (adds its tab), or folds the dock with null. */
export function setDrawerPane(id: string | null) {
  if (id) setDockTabs((t) => (t.includes(id) ? t : [...t, id]));
  showDocked(id);
}
export const [permissions, setPermissions] = createSignal<PermissionAsk[]>([]);
export const [screenPrompts, setScreenPrompts] = createSignal<ScreenPrompt[]>([]);

/** Panes of the active project (the workspace shows only these). */
export const projectPanes = createMemo(() => panes.filter((p) => !p.project || p.project === activeProject()));
export const panesOf = (project: string) => panes.filter((p) => p.project === project);
export const currentProject = createMemo(() => projects().find((p) => p.id === activeProject()) ?? null);

// --- Crew state, one per project; `crew` reads the active project's. ---
interface CrewState {
  phase: Phase;
  round: number;
  log: CrewEvent[];
  review: Review | null;
  gate: Gate | null;
}
const EMPTY: CrewState = { phase: "idle", round: 0, log: [], review: null, gate: null };
const [crews, setCrews] = createStore<Record<string, CrewState>>({});

/** Reactive view of the active project's crew state. */
export const crew: CrewState = new Proxy({} as CrewState, {
  get: (_, k: keyof CrewState) => {
    const id = activeProject();
    return (id && crews[id] ? crews[id][k] : EMPTY[k]) as never;
  },
});

export function setCrew(patch: Partial<CrewState>, project = activeProject()) {
  if (!project) return;
  setCrews(project, (c) => ({ ...(c ?? EMPTY), ...patch }));
}

export function pushCrew(ev: CrewEvent & { project?: string }) {
  const id = ev.project ?? activeProject();
  if (!id) return;
  if (!crews[id]) setCrews(id, { ...EMPTY, log: [] });
  setCrews(id, produce((c) => {
    c.phase = ev.phase;
    c.round = ev.round;
    c.log.push(ev);
    if (c.log.length > 200) c.log.splice(0, c.log.length - 200);
    if (ev.review) c.review = ev.review;
    if (ev.gate) c.gate = ev.gate;
  }));
}

export const crewPhaseOf = (project: string) => crews[project]?.phase ?? "idle";

// --- Frames: latest per pane (not reactive) + a per-pane version signal. ---
const frames = new Map<string, DataView>();
const versions = new Map<string, [() => number, (v: number) => void]>();

function version(id: string) {
  let v = versions.get(id);
  if (!v) {
    v = createSignal(0);
    versions.set(id, v);
  }
  return v;
}

export function pushFrame(id: string, frame: DataView) {
  frames.set(id, frame);
  const [get, set] = version(id);
  set(get() + 1);
}

/** Reactive accessor: re-runs when the pane gets a new frame. */
export function frameOf(id: string): DataView | undefined {
  version(id)[0]();
  return frames.get(id);
}

export function addPane(p: PaneView) {
  setPanes(produce((list) => {
    const i = list.findIndex((x) => x.id === p.id);
    if (i >= 0) list[i] = { ...list[i], ...p };
    else list.push(p);
  }));
  if (!focused() && (!p.project || p.project === activeProject())) setFocused(p.id);
}

export function removePane(id: string) {
  setPanes((list) => list.filter((p) => p.id !== id));
  frames.delete(id);
  versions.delete(id);
  if (focused() === id) setFocused(projectPanes()[0]?.id ?? null);
  if (maximized() === id) setMaximized(null);
}

export function setAgentState(key: string, state: AgentState) {
  setPanes((p) => p.key === key, "state", state);
}

export function markExited(id: string) {
  setPanes((p) => p.id === id, "exited", true);
}
