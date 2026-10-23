// Synthetic data shared by every direction, so only the design changes between
// previews. Nothing here comes from a real project or a real request.
// Every figure that appears twice is computed from one source below.

import type { Role as KitRole, Visual } from "../ui/kit/roles";

export type State = Visual;
export type LotStatus = "planned" | "working" | "needs" | "review" | "done";

export type Role = KitRole;
export type Agent = { key: string; name: string; role: Role; model: string; state: State; task: string; lot?: string; since: string };
export type Lot = { id: string; title: string; status: LotStatus; agent?: string; files: number; tag: string };

const clock = (m: number) => `${String(Math.floor(m / 60)).padStart(2, "0")}:${String(m % 60).padStart(2, "0")}`;
const at = (h: number, m: number) => h * 60 + m;

export const NOW = at(9, 50);

export const project = { name: "harness-test", scope: "sha-riyat", repo: "harnes/harness-test", branch: "run/contact-page", task: "Add a contact page with a form that sends an email" };
export const projects = [
  { name: "harness-test", active: true, need: 2 },
  { name: "site-vitrine", active: false, need: 0 },
  { name: "api-billing", active: false, need: 0 },
];
export const views = ["Chat", "Run", "Panes", "Methods", "Agents"] as const;

export const run = { started: at(9, 12), wave: 1, waves: 2 };
export const runElapsed = NOW - run.started;
export const minutes = (m: number) => (m < 60 ? `${m} min` : `${Math.floor(m / 60)} h ${String(m % 60).padStart(2, "0")}`);
export const time = clock;

export const agents: Agent[] = [
  { key: "orch", name: "Orchestrator", role: "orchestrator", model: "Sonnet 5.5", state: "working", task: "Waiting for wave 1", since: clock(at(9, 12)) },
  { key: "teach", name: "Teacher", role: "teacher", model: "Sonnet 5.5", state: "done", task: "Quality bar set for L0 to L2", since: clock(at(9, 19)) },
  { key: "plan", name: "Planner", role: "planner", model: "Opus 5.5", state: "done", task: `Plan ready, ${6} lots`, since: clock(at(9, 14)) },
  { key: "w1", name: "Worker L1", role: "worker", model: "Sonnet 5.5", state: "blocked", task: "Asks: SMTP or provider API?", lot: "L1", since: clock(at(9, 46)) },
  { key: "w2", name: "Worker L2", role: "worker", model: "Sonnet 5.5", state: "working", task: "Writing contact.html", lot: "L2", since: clock(at(9, 31)) },
  { key: "scout", name: "Scout", role: "scout", model: "Haiku 5.5", state: "working", task: "Reading the routes", lot: "L1", since: clock(at(9, 44)) },
  { key: "rev", name: "Reviewer L5", role: "reviewer", model: "Sonnet 5.5", state: "idle", task: "Waits for the diff", lot: "L5", since: clock(at(9, 40)) },
];

export const lots: Lot[] = [
  { id: "L0", title: "Header navigation", status: "done", agent: "w2", files: 3, tag: "ui" },
  { id: "L1", title: "Contact form and mail", status: "needs", agent: "w1", files: 4, tag: "feature" },
  { id: "L2", title: "Contact page layout", status: "working", agent: "w2", files: 2, tag: "ui" },
  { id: "L3", title: "Validation messages", status: "planned", files: 1, tag: "ui" },
  { id: "L4", title: "Mail provider config", status: "planned", files: 2, tag: "config" },
  { id: "L5", title: "Footer links", status: "review", agent: "rev", files: 1, tag: "ui" },
];
export const lotsDone = lots.filter((l) => l.status === "done").length;
console.assert(lots.length === 6, "the plan has 6 lots");

export const events: { at: number; who: string; text: string; ask?: boolean }[] = [
  { at: at(9, 47), who: "Worker L1", text: "asked to run npm install nodemailer", ask: true },
  { at: at(9, 46), who: "Worker L1", text: "asked: SMTP or provider API?", ask: true },
  { at: at(9, 44), who: "Scout", text: "started reading the routes for L1" },
  { at: at(9, 40), who: "Reviewer L5", text: "is waiting for the diff of L5" },
  { at: at(9, 31), who: "Worker L2", text: "started L2, Contact page layout" },
  { at: at(9, 24), who: "Worker L2", text: "finished L0, Header navigation, 3 files" },
  { at: at(9, 19), who: "Teacher", text: "set the quality bar for L0 to L2" },
  { at: at(9, 17), who: "Worker L2", text: "started L0, Header navigation" },
  { at: at(9, 14), who: "Planner", text: "plan ready, 6 lots in 2 waves" },
  { at: at(9, 12), who: "Orchestrator", text: "started the run" },
];

// What waits for the user. Waiting time is computed from the agent's own "since".
export const waiting = { agent: agents[2], sinceMin: at(9, 46) };
export const waited = NOW - waiting.sinceMin;

export const permission = { agent: "Worker L1", tool: "Bash", command: "npm install nodemailer", since: clock(at(9, 47)) };
export const question = {
  agent: "Worker L1",
  text: "Does the form send the mail through SMTP or the provider API?",
  since: clock(waiting.sinceMin),
  options: [
    { id: "smtp", label: "SMTP", detail: "nodemailer, already installed" },
    { id: "api", label: "Provider API", detail: "needs a key" },
  ],
};
export const needCount = 2;
console.assert(needCount === 2 && agents.filter((a) => a.state === "blocked").length === 1, "one agent blocked on two items");

// Five-hour window: usage in percent at a few instants, all before now.
export const windowUsage = {
  start: at(6, 42),
  end: at(11, 42),
  points: [[at(6, 42), 0], [at(8, 42), 3], [at(9, 12), 5], [at(9, 30), 15], [NOW, 27]] as [number, number][],
  events: [{ at: run.started, label: "Run started" }],
};
export const usage = { fiveHour: windowUsage.points.at(-1)![1] / 100, resetsIn: windowUsage.end - NOW, week: 0.65, weekResetsInDays: 3, saved: 203_000, requests: 42 };
console.assert(usage.resetsIn === 112, "window resets in 1 h 52");

export const models = [...new Set(agents.map((a) => a.model))].map((m) => ({ model: m, agents: agents.filter((a) => a.model === m).length }));

export const stateLabel: Record<State, string> = { working: "Working", blocked: "Needs you", idle: "Idle", done: "Done" };
export const lotLabel: Record<LotStatus, string> = { planned: "Planned", working: "Working", needs: "Needs you", review: "In review", done: "Done" };

// What the live panels replay. Times in the thread are real; a message that arrives
// during the replay is labelled "just now".
export type WorkStep = { kind: "read" | "edit" | "run"; label: string; result?: string; added?: number; removed?: number };
export const workLog: WorkStep[] = [
  { kind: "read", label: "Read src/pages/index.html" },
  { kind: "read", label: "Read src/styles/site.css" },
  { kind: "edit", label: "Edit src/pages/contact.html", added: 42, removed: 0 },
  { kind: "run", label: "Run npm test", result: "3 passed" },
  { kind: "edit", label: "Edit src/styles/site.css", added: 11, removed: 2 },
];
export const workAgent = agents[3];

export type Msg = { from: string; to: string; text: string; at: string; live?: boolean };
export const thread = {
  lot: "L1",
  title: "Contact form and mail",
  messages: [
    { from: "Planner", to: "Worker L1", text: "Feature. Files: contact.html and server/mail.js. The form must validate and send.", at: "09:15" },
    { from: "Worker L1", to: "Planner", text: "Which transport sends the mail? SMTP is installed, the provider API needs a key.", at: "09:46" },
    { from: "Scout", to: "Worker L1", text: "nodemailer is in package.json and .env has no provider key.", at: "just now", live: true },
    { from: "Orchestrator", to: "you", text: "L1 waits for your answer before it goes on.", at: "just now", live: true },
  ] as Msg[],
};

export const pulls = [
  { n: 14, lot: "L0", title: "Header navigation", branch: "lot/L0-header", state: "merged" as const, at: "09:24" },
  { n: 15, lot: "L5", title: "Footer links", branch: "lot/L5-footer", state: "open" as const, at: "09:41" },
];
export const checks = ["Build", "Tests", "Review"];

// A Worker thinking about its question, step by step. A step shows its active
// wording until `until`, then its done wording. Ticks are seconds of the replay.
export type ThinkStep = {
  id: string;
  kind: "search" | "read" | "dead" | "map" | "parallel" | "found" | "draft" | "end";
  active?: string;
  done: string;
  from: number;
  until: number;
  chips?: { name: string; at: number }[];
  subs?: { label: string; at: number }[];
  note?: string;
};
export const thinking: ThinkStep[] = [
  { id: "s1", kind: "search", active: "Searching the codebase", done: "Searched the codebase", from: 0, until: 3, chips: [{ name: "server/mail.js", at: 0 }, { name: "package.json", at: 1 }, { name: ".env.example", at: 1 }, { name: "Nodemailer docs", at: 2 }] },
  { id: "s2", kind: "read", active: "Reading 5 of 7 files", done: "Read 7 files about the mail transport", from: 3, until: 5 },
  { id: "s3", kind: "dead", done: "Dead end: the provider API needs a key that .env does not have", from: 5, until: 5 },
  { id: "s4", kind: "map", active: "Mapping how the form reaches the mail code", done: "Mapped 6 modules between the form and the mail code", from: 5, until: 7 },
  { id: "s5", kind: "parallel", active: "Running the form tests in parallel", done: "Merged the test results", from: 7, until: 10, subs: [{ label: "Ran 12 form tests", at: 8 }, { label: "3 tests touch mail.js", at: 9 }] },
  { id: "s6", kind: "found", done: "Found 2 ways to send: SMTP and the provider API", from: 10, until: 10 },
  { id: "s7", kind: "draft", active: "Drafting the question for you", done: "Drafted a question with 2 options", from: 11, until: 13 },
  { id: "end", kind: "end", done: "Finished thinking", from: 14, until: 14, note: "1 question for you · 2 options" },
];
export const THINK_END = 14;

// The context window of one agent, in thousands of tokens. Parts add up to the total.
export const contextWindow = {
  total: 1000,
  parts: [
    { id: "system", label: "System", k: 32.9, tip: "Instructions and role of the agent" },
    { id: "mcp", label: "MCP", k: 21.3, tip: "harnes-ctx, harnes-e2e · 2 servers" },
    { id: "skills", label: "Skills", k: 8.3, tip: "5 skills loaded for this lot" },
    { id: "messages", label: "Messages", k: 541.6, tip: "The conversation of this session" },
  ],
  reserved: 33,
  after: 14.2, // what the messages weigh once compacted
  session: 0.68,
  weekly: 0.24,
};
console.assert(Math.abs(contextWindow.parts.reduce((s, p) => s + p.k, 0) - 604.1) < 0.05, "context parts add up to 604.1k");
