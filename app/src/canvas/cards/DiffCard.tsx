// A diff written live. Lines arrive one per tick: a removed line is struck back,
// added lines settle with a quiet wash, and the counters in the header tick up.
// A second file joins at the end, folded.

import { For, Show } from "solid-js";
import { Card } from "@/components/ui/card";
import { cx } from "@/lib/cva";
import Icon from "../../ui/Icon";
import { ic } from "../icons";
import AgentIcon from "../ui/AgentIcon";
import { createSim } from "../ui/sim";
import { diff, diffFile } from "./cardData";

export default function DiffCard() {
  const sim = createSim(900, diff.length + 4, 4);
  const t = sim.t;
  const shown = () => diff.filter((_, i) => t() > i);
  const added = () => shown().filter((l) => l.kind === "add").length;
  const removed = () => shown().filter((l) => l.kind === "del").length;
  const writing = () => t() <= diff.length;
  return (
    <Card class="gap-0 rounded-xl py-0 shadow-none">
      <header class="flex items-center gap-2.5 border-b px-4 py-3">
        <AgentIcon role="worker" state={writing() ? "working" : "done"} />
        <div class="min-w-0 flex-1 leading-tight"><p class="truncate font-mono text-sm">{diffFile}</p><p class="text-xs text-muted-foreground">Worker L2 · lot L2</p></div>
        <p class="num flex gap-2 font-mono text-xs">
          <Show when={added()} keyed>{(n) => <span class="m-tick">+{n}</span>}</Show>
          <Show when={removed()} keyed>{(n) => <span class="m-tick text-muted-foreground">−{n}</span>}</Show>
        </p>
      </header>
      <ol class="min-h-[236px] py-2 font-mono text-[12.5px] leading-6">
        <For each={shown()}>
          {(l, i) => (
            <li class={cx("flex gap-3 px-4", l.kind === "add" && "m-rise m-wash bg-muted/60", l.kind === "del" && "m-rise text-muted-foreground")}>
              <span class="num w-5 shrink-0 text-right text-muted-foreground/70">{l.n}</span>
              <span class="w-3 shrink-0 text-muted-foreground">{l.kind === "add" ? "+" : l.kind === "del" ? "−" : ""}</span>
              <span class={cx("whitespace-pre", l.kind === "del" && "line-through decoration-muted-foreground/60", writing() && i() === shown().length - 1 && "m-caret")}>{l.text}</span>
            </li>
          )}
        </For>
      </ol>
      <Show when={t() > diff.length + 1}>
        <div class="m-rise flex items-center gap-2.5 border-t px-4 py-2.5 text-sm">
          <Icon icon={ic.file} size={15} class="text-muted-foreground" />
          <span class="flex-1 font-mono text-[13px]">src/styles/site.css</span>
          <span class="num font-mono text-xs">+11 <span class="text-muted-foreground">−2</span></span>
          <Icon icon={ic.caret} size={14} class="-rotate-90 text-muted-foreground" />
        </div>
      </Show>
    </Card>
  );
}
