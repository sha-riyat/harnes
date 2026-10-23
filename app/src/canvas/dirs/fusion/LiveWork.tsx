// What an agent is doing, one line at a time. The line being written is
// uncovered left to right with a caret; when its step ends the line settles,
// a check draws itself and the result appears. Nothing else moves.

import { For, Show } from "solid-js";
import { Card } from "@/components/ui/card";
import { cx } from "@/lib/cva";
import Icon from "../../../ui/Icon";
import { workAgent, workLog, type WorkStep } from "../../data";
import { ic } from "../../icons";
import { StateDot } from "../../ui/parts";

const icon: Record<WorkStep["kind"], string> = { read: ic.file, edit: ic.pencil, run: ic.terminal };
const APPEAR = (i: number) => 1 + i * 2; // tick at which line i starts
const DONE = (i: number) => APPEAR(i) + 1; // tick at which it ends

function Check() {
  return (
    <svg viewBox="0 0 16 16" class="size-4 text-foreground" aria-label="done">
      <path d="M3.5 8.5l3 3 6-7" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" class="m-draw" style={{ "--len": "16", "--dur": "220ms" }} />
    </svg>
  );
}

export default function LiveWork(props: { t: () => number }) {
  const shown = () => workLog.filter((_, i) => props.t() >= APPEAR(i));
  const finished = () => props.t() >= DONE(workLog.length - 1);
  return (
    <Card class="min-h-0 gap-0 rounded-lg py-0 shadow-none">
      <header class="flex items-center gap-2.5 border-b px-4 py-2.5">
        <StateDot state={finished() ? "idle" : "working"} />
        <h2 class="text-sm font-semibold">{workAgent.name}</h2>
        <p class="min-w-0 flex-1 truncate text-sm text-muted-foreground">L2 · Contact page layout</p>
        <span class="num text-xs text-muted-foreground">{finished() ? "step done" : "live"}</span>
      </header>
      <ol class="flex min-h-0 flex-1 flex-col gap-1 px-4 py-3">
        <For each={shown()}>
          {(s, i) => {
            const active = () => props.t() < DONE(i());
            return (
              <li class="m-rise flex h-7 items-center gap-2.5 text-sm">
                <span class="grid size-4 place-items-center">
                  <Show when={active()} fallback={<Check />}>
                    <span class="m-breathe size-1.5 rounded-full bg-foreground" />
                  </Show>
                </span>
                <Icon icon={icon[s.kind]} size={15} class="text-muted-foreground" />
                <span class={cx("min-w-0 truncate", active() ? "m-caret" : "text-muted-foreground")}>
                  <Show when={active()} fallback={s.label}>
                    <span class="m-reveal" style={{ "--n": s.label.length }}>{s.label}</span>
                  </Show>
                </span>
                <Show when={!active() && s.added !== undefined}>
                  <span class="m-pop num ml-auto font-mono text-xs"><span class="text-foreground">+{s.added}</span> <span class="text-muted-foreground">−{s.removed}</span></span>
                </Show>
                <Show when={!active() && s.result}>
                  <span class="m-pop num ml-auto rounded-full bg-muted px-2 py-0.5 text-xs">{s.result}</span>
                </Show>
              </li>
            );
          }}
        </For>
        <Show when={props.t() === 0}><li class="text-sm text-muted-foreground">Reading the plan…</li></Show>
      </ol>
    </Card>
  );
}
