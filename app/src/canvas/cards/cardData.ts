// Synthetic data for the cards that tell one story each. Same crew, same project
// as data.ts; nothing here comes from a real run.

// The plan: lots in waves, and what each later lot waits for.
export const waves = [
  { n: 1, lots: ["L0", "L1", "L2", "L5"] },
  { n: 2, lots: ["L3", "L4"] },
];
export const deps: [from: string, to: string][] = [["L2", "L3"], ["L1", "L4"]];

// Pull request #15 of lot L5, newest commit first.
export const pr = {
  n: 15,
  title: "Footer links",
  branch: "lot/L5-footer",
  base: "main",
  commits: [
    { sha: "d29f58", msg: "Add footer links", who: "Worker L2", at: "09:41" },
    { sha: "5be27d", msg: "Read the links from the site config", who: "Worker L2", at: "09:35" },
    { sha: "a3f9c1", msg: "Rename the footer partial", who: "Worker L2", at: "09:30" },
    { sha: "9c2d4a", msg: "Bump next to 16.3", who: "main", at: "09:00", base: true },
  ],
  mergeCommit: { sha: "7e0c3b", msg: "Merge main into lot/L5-footer", who: "Reviewer L5", at: "now" },
  conflicts: [
    { file: "footer.html", line: 12, theirs: '<a href="/legal">Legal</a>', ours: '<a href="/privacy">Privacy</a>' },
    { file: "site.css", line: 40, theirs: "gap: 1.5rem;", ours: "gap: 1rem;" },
  ],
  checks: ["Build", "Tests · 12 passed", "Lint", "Preview ready"],
};

// A diff written live by Worker L2.
export type DiffLine = { n: number; kind: "ctx" | "add" | "del"; text: string };
export const diffFile = "src/pages/contact.html";
export const diff: DiffLine[] = [
  { n: 8, kind: "ctx", text: "<main>" },
  { n: 9, kind: "del", text: "  <p>The contact page is coming soon.</p>" },
  { n: 9, kind: "add", text: '  <form class="contact" method="post" action="/api/contact">' },
  { n: 10, kind: "add", text: '    <label for="email">Email</label>' },
  { n: 11, kind: "add", text: '    <input id="email" name="email" type="email" required>' },
  { n: 12, kind: "add", text: '    <label for="message">Message</label>' },
  { n: 13, kind: "add", text: '    <textarea id="message" name="message" rows="5" required></textarea>' },
  { n: 14, kind: "add", text: '    <button type="submit">Send</button>' },
  { n: 15, kind: "add", text: "  </form>" },
  { n: 16, kind: "ctx", text: "</main>" },
];

export const suites = [
  { name: "form validation", tests: 6, ms: 900 },
  { name: "mail transport", tests: 3, ms: 2100, note: "1 retry" },
  { name: "layout", tests: 3, ms: 1200 },
];
export const testsTotal = suites.reduce((s, x) => s + x.tests, 0);

export const trust = {
  files: 14,
  findings: [
    { sev: "High", file: ".claude/settings.json", msg: "enableAllProjectMcpServers is on and .mcp.json also declares servers: they start without asking." },
    { sev: "Medium", file: ".mcp.json", msg: "One server runs npx without a pinned version." },
  ],
  fingerprint: "ab12",
};

export const gate = {
  summary: "Contact page with a working form. All 6 lots are merged and the 12 tests pass. The mail goes through SMTP, as you chose.",
  stats: [
    { label: "Lots merged", value: "6", unit: "/ 6" },
    { label: "Files changed", value: "14" },
    { label: "Lines", value: "+312", unit: "−41" },
    { label: "Tests", value: "12", unit: "passed" },
  ],
};
