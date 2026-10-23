// How an agent shows up: its role, its name and how its state reads, taken from the
// pane (key, role, state) that harnes already keeps. No new backend data.

import type { AgentState as PaneState } from "../../api";

export type Role = "orchestrator" | "planner" | "teacher" | "worker" | "scout" | "reviewer";
export type Visual = "working" | "blocked" | "idle" | "done";

const KNOWN: Role[] = ["orchestrator", "planner", "teacher", "worker", "scout", "reviewer"];

/** The role of a pane: its declared role, or what its key says. Plain sessions read as workers. */
export function roleOf(key?: string, role?: string): Role {
  if (role && (KNOWN as string[]).includes(role)) return role as Role;
  const k = key ?? "";
  if (k.startsWith("orch-")) return "orchestrator";
  for (const r of KNOWN) if (k.includes(`-${r}`)) return r;
  return "worker";
}

/** "17-worker-L1-r0" -> "Worker L1", "orch-ab12" -> "Orchestrator". */
export function nameOf(key?: string, title?: string): string {
  const k = key ?? title ?? "";
  if (k.startsWith("orch-")) return "Orchestrator";
  const clean = k.replace(/^\d+-/, "").replace(/-r\d+$/, "").replace(/\/.*$/, (m) => ` ${m.slice(1)}`).replace(/-/g, " ");
  return clean ? clean[0].toUpperCase() + clean.slice(1) : "Session";
}

/** The lot an agent works on ("L1"), when its key names one. */
export const lotOf = (key?: string) => key?.match(/-(L\d+)(?:-|$)/)?.[1];

/** Pane states reduce to the four the interface draws. */
export function visual(state?: PaneState, exited?: boolean): Visual {
  if (exited || state === "exited") return "done";
  if (state === "working" || state === "starting") return "working";
  if (state === "blocked") return "blocked";
  if (state === "done") return "done";
  return "idle";
}

export const roleHint: Record<Role, string> = {
  orchestrator: "Steers the run",
  planner: "Cuts the plan",
  teacher: "Sets the bar",
  worker: "Builds a lot",
  scout: "Reads and finds",
  reviewer: "Checks the diff",
};

export const visualLabel: Record<Visual, string> = { working: "Working", blocked: "Needs you", idle: "Idle", done: "Done" };
