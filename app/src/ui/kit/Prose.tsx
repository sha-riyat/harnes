// A long agent message, laid out to be read: lead paragraph, labelled sections,
// numbered items with their own marker, tokens set in mono. See lib/prose.ts.

import { For, Match, Switch, createMemo } from "solid-js";
import { cx } from "@/lib/cva";
import { parse, type Inline } from "../../lib/prose";

const Parts = (props: { parts: Inline[] }) => (
  <For each={props.parts}>{(p) => (p.code ? <code class="rounded bg-muted px-[3px] py-px font-mono text-[12px] [overflow-wrap:anywhere]">{p.s}</code> : p.s)}</For>
);

export default function Prose(props: { text: string; class?: string }) {
  const blocks = createMemo(() => parse(props.text));
  return (
    <div class={cx("flex flex-col gap-2.5 break-words text-[13.5px] leading-[1.6]", props.class)}>
      <For each={blocks()}>
        {(b) => (
          <Switch>
            <Match when={b.t === "p" && b}>{(p) => <p><Parts parts={p().parts} /></p>}</Match>
            <Match when={b.t === "label" && b}>{(l) => <p class="mt-1.5 text-xs font-medium text-muted-foreground">{l().text}</p>}</Match>
            <Match when={b.t === "list" && b}>
              {(l) => (
                <ol class="flex flex-col gap-2">
                  <For each={l().items}>
                    {(it, i) => (
                      <li class="flex gap-2.5">
                        <span class="num mt-[3px] grid size-[18px] shrink-0 place-items-center rounded-full bg-muted text-[11px] text-muted-foreground">{l().ordered ? i() + 1 : "–"}</span>
                        <span class="min-w-0 flex-1"><Parts parts={it} /></span>
                      </li>
                    )}
                  </For>
                </ol>
              )}
            </Match>
            <Match when={b.t === "code" && b}>{(c) => <pre class="overflow-x-auto rounded-lg bg-muted px-3 py-2 font-mono text-xs leading-relaxed">{c().text}</pre>}</Match>
          </Switch>
        )}
      </For>
    </div>
  );
}
