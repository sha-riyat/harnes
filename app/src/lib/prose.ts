// Reads a long agent message into blocks a person can scan: a lead, labelled
// sections, numbered items, paragraphs of a few sentences, and code-like tokens set
// apart. It only rearranges: every character of the message is still shown.

export type Inline = { s: string; code?: boolean };
export type Block =
  | { t: "p"; parts: Inline[] }
  | { t: "label"; text: string }
  | { t: "list"; ordered: boolean; items: Inline[][] }
  | { t: "code"; text: string };

const TOKEN = new RegExp(
  [
    "`[^`\\n]+`",
    "\\b(?:aria|data)-[\\w-]+(?:=\"[^\"]*\")?",
    "\\.?[\\w-]+(?:/[\\w.-]+)*/[\\w-]+(?:\\.[\\w-]+)*\\.\\w+",
    "\\.[\\w-]+(?:/[\\w.-]+)+",
    "\\b[\\w-]+\\.(?:html|css|js|ts|tsx|json|ya?ml|md|rs|toml|svg|png)\\b",
    "\\bU\\+[0-9A-F]{4,6}\\b",
    "&#?\\w+;",
    "\\b[a-z][a-z0-9]*_[a-z0-9_]+\\b",
    "\\bharnes-[\\w-]+",
  ].join("|"),
  "g",
);

export function inline(s: string): Inline[] {
  const out: Inline[] = [];
  let last = 0;
  for (const m of s.matchAll(TOKEN)) {
    const at = m.index ?? 0;
    if (at > last) out.push({ s: s.slice(last, at) });
    out.push({ s: m[0].startsWith("`") ? m[0].slice(1, -1) : m[0], code: true });
    last = at + m[0].length;
  }
  if (last < s.length) out.push({ s: s.slice(last) });
  return out;
}

const LABEL = /(?:^|(?<=[.!?;)]\s))((?:[A-ZÀ-Ý][A-ZÀ-Ý0-9_' -]{3,})|Fichiers|Files|Objectif|Goal|Contexte|Context|Contraintes|Constraints|Livrable|Deliverable|R[ée]sultat|Result|V[ée]rifier|Verify|Note|Tests?)\s?:\s+/g;
const FENCE = /```[\w-]*\n([\s\S]*?)```/g;
const BULLET = /^\s*(?:([-*•])|(\d+)[.)])\s+/;
const SENTENCE = /(?<=[.!?])\s+(?=[A-ZÀ-Ý(])/;

const sentenceCase = (s: string) => (s === s.toUpperCase() ? s[0] + s.slice(1).toLowerCase() : s);

function sections(par: string) {
  const out: { label?: string; text: string }[] = [];
  let last = 0;
  let label: string | undefined;
  for (const m of par.matchAll(LABEL)) {
    const at = m.index ?? 0;
    const before = par.slice(last, at).trim();
    if (before || label) out.push({ label, text: before });
    label = sentenceCase(m[1]);
    last = at + m[0].length;
  }
  out.push({ label, text: par.slice(last).trim() });
  return out.filter((x) => x.text || x.label);
}

/** "(1) a ; (2) b" → the text before, the items, and what follows the last one. */
function enumerate(text: string) {
  const parts = text.split(/\s*\((\d{1,2})\)\s+/);
  if (parts.length < 5 || parts[1] !== "1" || parts[3] !== "2") return null;
  const items: string[] = [];
  let rest = "";
  for (let i = 2; i < parts.length; i += 2) {
    let item = parts[i].replace(/\s*;\s*$/, "");
    if (i + 2 >= parts.length) {
      const cut = item.search(/\.\s+(?=[A-ZÀ-Ý])/);
      if (cut > 0) (rest = item.slice(cut + 1).trim(), (item = item.slice(0, cut + 1)));
    }
    items.push(item);
  }
  return { intro: parts[0].trim(), items, rest };
}

function body(text: string, out: Block[]) {
  const e = enumerate(text);
  if (e) {
    if (e.intro) out.push({ t: "p", parts: inline(e.intro) });
    out.push({ t: "list", ordered: true, items: e.items.map(inline) });
    if (e.rest) body(e.rest, out);
    return;
  }
  if (text.length <= 360) return void out.push({ t: "p", parts: inline(text) });
  let cur = "";
  for (const s of text.split(SENTENCE)) {
    cur = cur ? `${cur} ${s}` : s;
    if (cur.length >= 220) (out.push({ t: "p", parts: inline(cur) }), (cur = ""));
  }
  if (cur) out.push({ t: "p", parts: inline(cur) });
}

function prose(chunk: string, out: Block[]) {
  let run: { ordered: boolean; items: string[] } | null = null;
  const flush = () => run && (out.push({ t: "list", ordered: run.ordered, items: run.items.map(inline) }), (run = null));
  for (const line of chunk.split("\n")) {
    const b = line.match(BULLET);
    if (b) {
      const ordered = !!b[2];
      if (run && run.ordered !== ordered) flush();
      (run ??= { ordered, items: [] }).items.push(line.replace(BULLET, "").trim());
      continue;
    }
    flush();
    if (!line.trim()) continue;
    for (const sec of sections(line.trim())) {
      if (sec.label) out.push({ t: "label", text: sec.label });
      if (sec.text) body(sec.text, out);
    }
  }
  flush();
}

export function parse(text: string): Block[] {
  const out: Block[] = [];
  let last = 0;
  for (const m of text.matchAll(FENCE)) {
    prose(text.slice(last, m.index), out);
    out.push({ t: "code", text: m[1].replace(/\n$/, "") });
    last = (m.index ?? 0) + m[0].length;
  }
  prose(text.slice(last), out);
  return out;
}

const plain = (parts: Inline[]) => parts.map((p) => p.s).join("");

/** What a folded message shows: its first full sentence, and the sections it holds. */
export function digest(blocks: Block[]) {
  // A label's value ("Fichiers : a.html, b.css") is not a sentence to lead with.
  const sentences = blocks.flatMap((b, i) => (b.t === "p" && blocks[i - 1]?.t !== "label" ? plain(b.parts).split(SENTENCE) : []));
  const lead = (sentences.find((s) => s.length >= 40) ?? sentences[0] ?? "").replace(/\s*:$/, "");
  const labels = blocks.flatMap((b) => (b.t === "label" ? [b.text] : []));
  const items = blocks.reduce((n, b) => n + (b.t === "list" ? b.items.length : 0), 0);
  return { lead, labels, items };
}

export const words = (text: string) => text.trim().split(/\s+/).filter(Boolean).length;
/** A message this long is folded to its digest unless it is the latest. */
export const isLong = (text: string) => text.length > 520;
