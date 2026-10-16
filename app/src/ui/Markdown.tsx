// A small Markdown renderer for agent messages: headings, lists, quotes,
// fenced code, rules, and inline bold, italic, code and links. It builds
// nodes, never HTML strings, so a message cannot inject markup.

import { For, Match, Switch, type JSX } from "solid-js";

type Block =
  | { kind: "p"; text: string }
  | { kind: "h"; level: number; text: string }
  | { kind: "code"; text: string }
  | { kind: "quote"; text: string }
  | { kind: "list"; ordered: boolean; items: string[] }
  | { kind: "rule" };

const BULLET = /^\s*[-*+]\s+(.*)$/;
const NUMBER = /^\s*\d+[.)]\s+(.*)$/;

export function blocks(src: string): Block[] {
  const out: Block[] = [];
  const lines = src.replace(/\r/g, "").split("\n");
  let i = 0;
  while (i < lines.length) {
    const l = lines[i];
    if (!l.trim()) { i++; continue; }
    if (l.trimStart().startsWith("```")) {
      const body: string[] = [];
      for (i++; i < lines.length && !lines[i].trimStart().startsWith("```"); i++) body.push(lines[i]);
      out.push({ kind: "code", text: body.join("\n") });
      i++;
      continue;
    }
    const h = /^(#{1,6})\s+(.*)$/.exec(l);
    if (h) { out.push({ kind: "h", level: h[1].length, text: h[2] }); i++; continue; }
    if (/^\s*([-*_])(\s*\1){2,}\s*$/.test(l)) { out.push({ kind: "rule" }); i++; continue; }
    if (l.startsWith(">")) {
      const body: string[] = [];
      for (; i < lines.length && lines[i].startsWith(">"); i++) body.push(lines[i].replace(/^>\s?/, ""));
      out.push({ kind: "quote", text: body.join("\n") });
      continue;
    }
    const ordered = NUMBER.test(l);
    if (ordered || BULLET.test(l)) {
      const re = ordered ? NUMBER : BULLET;
      const items: string[] = [];
      for (; i < lines.length; i++) {
        const m = re.exec(lines[i]);
        if (m) items.push(m[1]);
        else if (lines[i].startsWith("  ") && items.length) items[items.length - 1] += " " + lines[i].trim();
        else break;
      }
      out.push({ kind: "list", ordered, items });
      continue;
    }
    const body: string[] = [];
    for (; i < lines.length && lines[i].trim() && !/^(#{1,6}\s|```|>)/.test(lines[i]) && !BULLET.test(lines[i]) && !NUMBER.test(lines[i]); i++) body.push(lines[i]);
    out.push({ kind: "p", text: body.join("\n") });
  }
  return out;
}

const INLINE = /(`[^`]+`)|(\*\*[^*]+\*\*|__[^_]+__)|(\*[^*\s][^*]*\*|_[^_\s][^_]*_)|(\[[^\]]+\]\([^)\s]+\))/;

/** Inline spans: `code`, **bold**, *italic*, [text](url). */
export function inline(text: string): JSX.Element[] {
  const out: JSX.Element[] = [];
  let rest = text;
  while (rest) {
    const m = INLINE.exec(rest);
    if (!m) { out.push(rest); break; }
    if (m.index) out.push(rest.slice(0, m.index));
    const t = m[0];
    if (m[1]) out.push(<code>{t.slice(1, -1)}</code>);
    else if (m[2]) out.push(<strong>{inline(t.slice(2, -2))}</strong>);
    else if (m[3]) out.push(<em>{inline(t.slice(1, -1))}</em>);
    else {
      const [, label, href] = /^\[([^\]]+)\]\(([^)]+)\)$/.exec(t)!;
      out.push(/^https?:\/\//.test(href) ? <a href={href} target="_blank" rel="noreferrer">{label}</a> : <span class="md-ref">{label}</span>);
    }
    rest = rest.slice(m.index + t.length);
  }
  return out;
}

export default function Markdown(props: { text: string }) {
  return (
    <div class="md">
      <For each={blocks(props.text)}>
        {(b) => (
          <Switch>
            <Match when={b.kind === "p" && b}>{(p) => <p>{inline(p().text)}</p>}</Match>
            <Match when={b.kind === "h" && b}>{(h) => <p class={`md-h md-h${Math.min(h().level, 3)}`}>{inline(h().text)}</p>}</Match>
            <Match when={b.kind === "code" && b}>{(c) => <pre>{c().text}</pre>}</Match>
            <Match when={b.kind === "quote" && b}>{(q) => <blockquote>{inline(q().text)}</blockquote>}</Match>
            <Match when={b.kind === "rule"}><hr /></Match>
            <Match when={b.kind === "list" && b}>
              {(l) =>
                l().ordered ? (
                  <ol><For each={l().items}>{(it) => <li>{inline(it)}</li>}</For></ol>
                ) : (
                  <ul><For each={l().items}>{(it) => <li>{inline(it)}</li>}</For></ul>
                )
              }
            </Match>
          </Switch>
        )}
      </For>
    </div>
  );
}
