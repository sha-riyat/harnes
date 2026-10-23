// Synthetic content for the notification panel, the profile menu, Methods and
// Settings. Same crew and project as data.ts.

import type { Role } from "../data";

export type Notice = { id: string; role: Role; who: string; text: string; at: string; ask?: boolean; action?: string; unread: boolean; live?: boolean };

export const notices: Notice[] = [
  { id: "n1", role: "worker", who: "Worker L1", text: "asks: SMTP or the provider API?", at: "09:46", ask: true, action: "Answer", unread: true },
  { id: "n2", role: "worker", who: "Worker L1", text: "wants to run npm install nodemailer", at: "09:47", ask: true, action: "Review", unread: true },
  { id: "n3", role: "reviewer", who: "Reviewer L5", text: "has the diff of Footer links and is checking it", at: "09:41", unread: false },
  { id: "n4", role: "orchestrator", who: "Orchestrator", text: "merged pull request #14, Header navigation", at: "09:24", unread: false },
  { id: "n5", role: "planner", who: "Planner", text: "finished the plan: 6 lots in 2 waves", at: "09:14", unread: false },
];
export const liveNotice: Notice = { id: "n0", role: "scout", who: "Scout", text: "found nodemailer in package.json and no provider key", at: "just now", unread: true, live: true };

export const profile = {
  name: "sha-riyat",
  plan: "Claude Max",
  via: "connected through Claude Code",
  version: "harnes 0.1.0",
};

export type Pack = { id: string; purpose: string; roles: Role[]; skills: string[]; confidence: "A" | "B" };
export const packs: Pack[] = [
  { id: "orchestration-os", purpose: "Compile goals into workflows: DAGs, sagas, fanout and join, routing, compensation, timeouts.", roles: ["orchestrator"], skills: ["compile-workflow", "route-intent", "run-saga", "compensate"], confidence: "A" },
  { id: "agent-os", purpose: "Define agent lifecycle, roles, bounds, budgets, handoff and stop conditions.", roles: ["orchestrator"], skills: ["define-agent", "handoff"], confidence: "A" },
  { id: "project-os", purpose: "Bound work as a project: outcomes, constraints, risks and honest status.", roles: ["planner"], skills: ["charter", "status"], confidence: "A" },
  { id: "builder-os", purpose: "Construct working software carefully: construction plans, reviews, safe refactors, legacy seams.", roles: ["worker", "reviewer"], skills: ["construction-plan", "review-diff", "refactor-plan", "legacy-seam"], confidence: "A" },
  { id: "quality-evaluation-os", purpose: "Argue whether a product is good enough to ship.", roles: ["reviewer", "teacher"], skills: ["risk-charter", "test-design", "grade-build", "ship-readiness"], confidence: "A" },
  { id: "validation-os", purpose: "Turn hypotheses into experiments with kill rules.", roles: ["teacher"], skills: ["experiment", "kill-rule"], confidence: "B" },
  { id: "design-os", purpose: "Shape usable, ethical product experiences: flows, interaction specs, cheap usability.", roles: [], skills: ["job-frame", "critique-flow", "interaction-spec", "usability-lite"], confidence: "B" },
];
export const packText = {
  os: ["Purpose", "Construct working software carefully.", "Runtime loop", "1. Bind oracle/ORACLE.md as non-overridable policy.", "2. Route the intent through routing/intents.yaml.", "3. Hand the lot to a Worker, then to a Reviewer."],
  oracle: ["Defaults", "Small, reversible steps. Tests before refactor.", "Refuse", "Hero rewrites without seams."],
};

export type Setting =
  | { kind: "switch"; id: string; title: string; hint: string; on: boolean }
  | { kind: "model"; id: string; title: string; hint: string; role: Role; model: string; effort: string }
  | { kind: "choice"; id: string; title: string; hint: string; value: string; options: { id: string; label: string; hint: string }[] };

export const settingSections: { id: string; label: string; icon: "sliders" | "lockKey" | "lightning" | "bell" | "folder"; note: string; rows: Setting[] }[] = [
  {
    id: "crew", label: "Crew and models", icon: "sliders", note: "Which model each role uses, and how hard it thinks.",
    rows: [
      { kind: "model", id: "m-plan", title: "Planner", hint: "Cuts the plan. Used once per run.", role: "planner", model: "Opus 5.5", effort: "medium" },
      { kind: "model", id: "m-orch", title: "Orchestrator", hint: "Steers the run and talks to you.", role: "orchestrator", model: "Sonnet 5.5", effort: "high" },
      { kind: "model", id: "m-teach", title: "Teacher", hint: "Sets the quality bar for each lot.", role: "teacher", model: "Sonnet 5.5", effort: "high" },
      { kind: "model", id: "m-work", title: "Workers", hint: "Build the lots, one worktree each.", role: "worker", model: "Sonnet 5.5", effort: "high" },
      { kind: "model", id: "m-rev", title: "Reviewers", hint: "Check each diff before it merges.", role: "reviewer", model: "Sonnet 5.5", effort: "high" },
      { kind: "model", id: "m-scout", title: "Scouts", hint: "Read and find. Cheap and fast.", role: "scout", model: "Haiku 5.5", effort: "high" },
    ],
  },
  {
    id: "safety", label: "Safety", icon: "lockKey", note: "What harnes checks before and while the crew works.",
    rows: [
      { kind: "switch", id: "s-trust", title: "Trust scan before a run", hint: "Reads the project's settings and MCP servers and blocks the start on a finding.", on: true },
      { kind: "switch", id: "s-ask", title: "Ask before running commands", hint: "Every command waits for your answer unless you allowed it.", on: true },
      { kind: "switch", id: "s-tree", title: "One worktree per lot", hint: "Lots never touch each other's files.", on: true },
      { kind: "switch", id: "s-stall", title: "Stall detection", hint: "An agent that stops moving is flagged after 5 minutes.", on: true },
    ],
  },
  {
    id: "free", label: "Free models", icon: "lightning", note: "Let a free model propose; a paid one always checks.",
    rows: [
      { kind: "choice", id: "f-profile", title: "Profile", hint: "Nothing leaves your machine except to Claude unless you pick another.", value: "claude", options: [
        { id: "claude", label: "Claude only", hint: "Nothing goes to a third party" },
        { id: "mixed", label: "Mixed", hint: "Free scouts, Claude checks" },
        { id: "eco", label: "Max economy", hint: "Free first, Claude on failure" },
      ] },
      { kind: "switch", id: "f-zero", title: "Zero-retention models only", hint: "Models that keep prompts are never offered.", on: true },
    ],
  },
  {
    id: "notify", label: "Notifications", icon: "bell", note: "When harnes interrupts you, and how.",
    rows: [
      { kind: "switch", id: "n-ask", title: "A question or a permission", hint: "Always shown in the bell; here, also as a desktop notification.", on: true },
      { kind: "switch", id: "n-done", title: "A run is ready for you", hint: "Once, when the summary is written.", on: true },
      { kind: "switch", id: "n-fail", title: "A run stops", hint: "With the reason and the last good step.", on: true },
      { kind: "switch", id: "n-sound", title: "Sound", hint: "A short tick with each desktop notification.", on: false },
    ],
  },
  {
    id: "projects", label: "Projects", icon: "folder", note: "Folders harnes may open and work in.",
    rows: [
      { kind: "switch", id: "p-restore", title: "Restore sessions on start", hint: "Reopen panes and resume runs where they stopped.", on: true },
      { kind: "switch", id: "p-auto", title: "Open the last project", hint: "Skip the project picker.", on: false },
    ],
  },
];
