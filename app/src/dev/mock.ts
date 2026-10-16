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

const responses: Record<string, unknown> = {
  projects_list: projects,
  app_info: { project: "C:/work/harness-test", project_id: "p1", proxy_port: 62346, hook_port: 51743, ctx_bin: "", ctx_bin_found: true, crew_running: true },
  pane_list: panes,
  stats: { proxy: { requests: 42, errors: 0, bytes_saved: 812000, est_tokens_saved: 203000, elided_blocks: 7, input_tokens: 51000, output_tokens: 9100, cache_read_tokens: 380000, cache_write_tokens: 12000 }, ctx: { blobs: 12, bytes_stored: 2000000, bytes_returned: 40000 } },
  threads_list: threads,
  threads_recent: [{ id: 9, thread: "lot-1-L1", author: "1-worker-L1-r0/scout", at: now, kind: "message", body: "Endpoint trouvé" }],
  thread_read: { thread: threads[1], messages: [{ id: 1, thread: "lot-1-L1", author: "planner", at: now - 60000, kind: "handoff", body: "Lot L1 (feature). Fichiers : contact.html" }, { id: 2, thread: "lot-1-L1", author: "1-worker-L1-r0", at: now - 30000, kind: "message", body: "Quel endpoint pour l'envoi ?" }, { id: 3, thread: "lot-1-L1", author: "1-worker-L1-r0/scout", at: now - 5000, kind: "message", body: "Endpoint trouvé : POST /api/contact" }] },
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
