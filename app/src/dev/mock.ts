// Dev-only Tauri mock (http://localhost:1420/?mock) to work on the UI in a
// plain browser. Never bundled in production builds.

const projects = [
  { id: "p1", name: "harness-test", root: "C:/work/harness-test", active: true, running: true, phase: "working", panes: 3, working: 2, waiting: 1, proxy_port: 62346 },
  { id: "p2", name: "site-vitrine", root: "C:/work/site-vitrine", active: false, running: false, phase: null, panes: 1, working: 0, waiting: 0, proxy_port: 62400 },
  { id: "p3", name: "api-billing", root: "C:/work/api-billing", active: false, running: false, phase: null, panes: 0, working: 0, waiting: 0, proxy_port: 62411 },
];
const panes = [
  { id: "a1", title: "orchestrateur", program: "claude", cwd: "C:/work/harness-test", created: 1, project: "p1", key: "orch-p1", role: "orchestrator", provider: "claude", state: "working" },
  { id: "a2", title: "worker-L1-r0", program: "claude", cwd: "C:/work/harness-test", created: 2, project: "p1", key: "1-worker-L1-r0", role: "worker", provider: "claude", state: "blocked" },
  { id: "a3", title: "shell", program: "powershell.exe", cwd: "C:/work/harness-test", created: 3, project: "p1" },
  { id: "b1", title: "claude", program: "claude", cwd: "C:/work/site-vitrine", created: 4, project: "p2", key: "claude-abc", role: "claude", provider: "codex", state: "idle" },
];
const now = Date.now();
const threads = [
  { id: "run-1", title: "Ajouter une page contact", kind: "run", status: "open", messages: 6, last_at: now - 20000, last_author: "harnes", last_body: "Vague 1 : L1, L2" },
  { id: "lot-1-L1", title: "L1 · Formulaire de contact", kind: "lot", status: "open", messages: 3, last_at: now - 5000, last_author: "1-worker-L1-r0/scout", last_body: "Endpoint trouvé : POST /api/contact" },
  { id: "lot-0-L1", title: "L1 · Header", kind: "lot", status: "closed", messages: 4, last_at: now - 900000, last_author: "teacher", last_body: "Fil clos : lot terminé", summary: "lot terminé" },
];

const pack = (id: string, purpose: string, roles: string[], skills: string[]) => ({ id, title: id, version: "0.1.0", purpose, provides: skills, depends_on: [], confidence: "A", skills, agents: [], roles });
const packs = [
  pack("orchestration-os", "Compile goals into workflows: DAGs, sagas, fanout/join, routing, compensation, and timeouts.", ["orchestrator"], ["compile-workflow", "route-intent", "run-saga", "compensate"]),
  pack("agent-os", "Define agent lifecycle, roles, bounds, budgets, handoff, and stop conditions.", ["orchestrator"], ["define-agent", "handoff"]),
  pack("project-os", "Bound work as a project: outcomes, constraints, risks, and honest status.", ["planner"], ["charter", "status"]),
  pack("builder-os", "Construct working software carefully: construction plans, reviews, safe refactors, legacy seams.", ["worker", "reviewer"], ["construction-plan", "review-diff", "refactor-plan", "legacy-seam"]),
  pack("quality-evaluation-os", "Argue whether a product is good enough to ship.", ["reviewer", "teacher"], ["risk-charter", "test-design", "grade-build", "ship-readiness"]),
  pack("validation-os", "Turn hypotheses into experiments with kill rules.", ["teacher"], ["experiment", "kill-rule"]),
  pack("design-os", "Shape usable, ethical product experiences: flows, interaction specs, cheap usability.", [], ["job-frame", "critique-flow", "interaction-spec", "usability-lite"]),
  pack("pricing-os", "Design value-based price architecture: metrics, fences, versions, and ethics.", [], ["price-metric", "fences"]),
];

const responses: Record<string, unknown> = {
  os_list: { dir: "C:/Users/me/AppData/Roaming/dev.harnes.app/os", packs },
  os_pack: { pack: packs[3], os: ["# Builder OS", "", "## Purpose", "Construct working software carefully.", "", "## Runtime loop", "1. Bind oracle/ORACLE.md as non-overridable policy.", "2. Route via routing/intents.yaml."].join(String.fromCharCode(10)), oracle: ["# Oracle — builder-os", "", "## Defaults", "- Small reversible steps.", "- Tests before refactor.", "", "## Refuse", "- Hero rewrites without seams."].join(String.fromCharCode(10)) },
  projects_list: projects,
  app_info: { project: "C:/work/harness-test", project_id: "p1", proxy_port: 62346, hook_port: 51743, ctx_bin: "", ctx_bin_found: true, crew_running: true },
  pane_list: panes,
  stats: { proxy: { requests: 42, errors: 0, bytes_saved: 812000, est_tokens_saved: 203000, elided_blocks: 7, input_tokens: 51000, output_tokens: 9100, cache_read_tokens: 380000, cache_write_tokens: 12000 }, ctx: { blobs: 12, bytes_stored: 2000000, bytes_returned: 40000 }, runs: { runs: 6, passed: 5, first_pass: 4, mean_rounds: 1.33, important: 7, nits: 2, pre_existing: 3, resolved: { fixed: 6, wrong: 1 } } },
  profile_get: { config: { profile: "mixed", auto_escalate: true, models: ["opencode/glm-5.3-flash-free"], digest: true }, effective: "max-eco", opencode: true, live: ["opencode/glm-5.3-flash-free", "opencode/big-pickle"], usable: ["opencode/glm-5.3-flash-free"] },
  profile_set: { config: { profile: "mixed", auto_escalate: true, models: ["opencode/glm-5.3-flash-free"], digest: true }, effective: "max-eco", opencode: true, live: ["opencode/glm-5.3-flash-free"], usable: ["opencode/glm-5.3-flash-free"] },
  trust_report: { findings: [{ id: "auto-starts-mcp-servers", severity: "high", file: ".claude/settings.json", message: "`enableAllProjectMcpServers` is on and `.mcp.json` also declares theirs: they start without asking.", fingerprint: "ab12", blocks: true }], blocked: true },
  trust_accept: { findings: [], blocked: false },
  budget_get: { limits: { five_hour: 0.27, five_hour_reset: Math.round(now / 1000) + 7200, seven_day: 0.65, seven_day_reset: Math.round(now / 1000) + 3 * 86400, status: "allowed", seen: Math.round(now / 1000) }, week_elapsed: 0.57, eco: true },
  threads_list: threads,
  threads_recent: [{ id: 9, thread: "lot-1-L1", author: "1-worker-L1-r0/scout", at: now, kind: "message", body: "Endpoint trouvé" }],
  thread_read: { thread: threads[1], messages: [{ id: 1, thread: "lot-1-L1", author: "planner", at: now - 60000, kind: "handoff", body: "Lot L1 (feature). Fichiers : contact.html" }, { id: 2, thread: "lot-1-L1", author: "1-worker-L1-r0", at: now - 30000, kind: "message", body: "Quel endpoint pour l'envoi ?" }, { id: 3, thread: "lot-1-L1", author: "1-worker-L1-r0/scout", at: now - 5000, kind: "message", body: "Endpoint trouvé : POST /api/contact" }] },
  conversation: { started: true, state: "working", pane: "a1", items: [
    { role: "user", text: "Ajoute une page contact avec un formulaire qui envoie un e-mail.", at: new Date(now - 300000).toISOString() },
    { role: "assistant", text: "Je lance l'équipage : le planner découpe, le teacher fixe la barre, puis les workers construisent et testent.", at: new Date(now - 290000).toISOString() },
    { role: "tool", tool: "crew_start", text: "Page contact avec formulaire et envoi d'e-mail", at: new Date(now - 285000).toISOString() },
    { role: "tool", tool: "thread_read", text: "lot-1-L1", at: new Date(now - 60000).toISOString() },
    { role: "assistant", text: "Le plan compte 2 lots en une vague. worker-L1 a une question sur l'endpoint : un scout l'a trouvé (POST /api/contact), c'est intégré.", at: new Date(now - 50000).toISOString() },
    { role: "assistant", text: ["**Calculatrice web vanilla, 6 commits.**", "", "- `index.html` + `styles.css` : page calculatrice", "- `app.js` : logique (opérations, reset après `=`)", "", "Pas lancé les tests — *état réel non vérifié*."].join(String.fromCharCode(10)), at: new Date(now - 40000).toISOString() },
  ] },
  permissions_pending: [{ id: "x1", agent: "1-worker-L1-r0", tool: "Bash", summary: "npm install nodemailer", input: {}, can_remember: true, at: now }],
  screen_prompts: [{ pane: "a2", agent: "1-worker-L2-r0", project: "p1", prompt: { question: "Le fichier contact.html existe déjà. Que faire ?", choices: [{ keys: "1", label: "Remplacer" }, { keys: "2", label: "Garder les deux" }, { keys: "3", label: "Annuler" }] } }],
  providers_list: ["claude", "codex", "gemini", "opencode"],
  lookout: { snapshot: { generated_at: now, sessions: [], sources: [] }, events: [], history: [] },
};

let cb = 0;
(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
  transformCallback: () => ++cb,
  unregisterCallback: () => {},
  invoke: async (cmd: string) => (cmd in responses ? structuredClone(responses[cmd]) : null),
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
};
(window as unknown as Record<string, unknown>).__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
export {};
