// How an agent shows its thinking (after the "Thinking process" reference). One
// timeline: the step being worked is written out and breathes, finished steps
// settle to grey, a dead end is struck back, parallel work folds into one merge,
// and the last step ends on a check. The result of the thinking, a typed question
// for the user, then rises under it. Replays on its own.

import { For, Match, Show, Switch, createSignal } from "solid-js";
import { Card } from "@/components/ui/card";
import { cx } from "@/lib/cva";
import Icon from "../../../ui/Icon";
import { THINK_END, thinking, type ThinkStep } from "../../data";
import { ic } from "../../icons";
import { AskCard } from "../../ui/Decision";
import { createSim } from "../../ui/sim";

function Node(props: { step: ThinkStep; active: boolean }) {
  return (
    <span class="grid size-5 shrink-0 place-items-center">
      <Switch>
        <Match when={props.step.kind === "end"}>
          <span class="m-pop grid size-4 place-items-center rounded-full bg-foreground text-background">
            <svg viewBox="0 0 16 16" class="size-3"><path d="M3.5 8.5l3 3 6-7" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="m-draw" style={{ "--len": "16", "--dur": "220ms" }} /></svg>
          </span>
        </Match>
        <Match when={props.step.kind === "dead"}>
          <svg viewBox="0 0 10 10" class="size-2 text-muted-foreground/70"><path d="M1.5 1.5l7 7M8.5 1.5l-7 7" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" /></svg>
        </Match>
        <Match when={props.active}>
          <span class="m-breathe size-2.5 rounded-full border-2 border-foreground" />
        </Match>
        <Match when={props.step.kind === "search"}><Icon icon={ic.search} size={14} class="text-muted-foreground" /></Match>
        <Match when={props.step.kind === "parallel"}><span class="size-2 rounded-full border border-muted-foreground" /></Match>
        <Match when={true}><span class="size-1.5 rounded-full bg-muted-foreground/70" /></Match>
      </Switch>
    </span>
  );
}

export function ThinkingCard(props: { withQuestion?: boolean }) {
  const sim = createSim(1000, THINK_END + 1, 7);
  const t = sim.t;
  const [open, setOpen] = createSignal(true);
  const visible = () => thinking.filter((s) => t() >= s.from);
  const finished = () => t() >= THINK_END;
  return (
    <div class="flex flex-col gap-4">
      <div class="flex flex-col">
        <button class="flex items-center gap-2 self-start text-sm" onClick={() => setOpen(!open())} aria-expanded={open()}>
          <Icon icon={ic.caret} size={14} class={cx("text-muted-foreground transition-transform duration-200", !open() && "-rotate-90")} />
          <span class={cx("font-medium", finished() && "text-muted-foreground")}>{finished() ? "Thought for" : "Thinking"}</span>
          <span class="num text-muted-foreground">{Math.min(t(), THINK_END)}s</span>
        </button>
        <div class={cx("grid transition-[grid-template-rows] duration-200 ease-out", open() ? "grid-rows-[1fr]" : "grid-rows-[0fr]")}>
          <ol class="min-h-0 overflow-hidden pt-3">
            <For each={visible()}>
              {(s, i) => {
                const active = () => s.active !== undefined && t() < s.until;
                return (
                  <li class="m-rise relative flex gap-3 pb-3.5">
                    <Show when={i() < visible().length - 1}><span class="absolute top-5 -bottom-0.5 left-[9.5px] w-px bg-border" /></Show>
                    <Node step={s} active={active()} />
                    <div class="min-w-0 flex-1 text-sm leading-5">
                      <Show when={s.kind === "end"} fallback={
                        <p class={cx(active() ? "text-foreground" : s.kind === "dead" ? "text-muted-foreground/70" : "text-muted-foreground")}>
                          <Show when={active()} keyed fallback={s.done}>
                            <span class="m-reveal" style={{ "--n": s.active!.length }}>{s.active}</span>
                          </Show>
                        </p>
                      }>
                        <p><span class="font-medium">{s.done}</span> <span class="text-muted-foreground">{s.note}</span></p>
                      </Show>
                      <Show when={s.chips}>
                        <div class="mt-2 flex flex-wrap gap-1.5">
                          <For each={s.chips!.filter((c) => t() >= c.at)}>
                            {(c) => <span class="m-pop inline-flex h-6 items-center gap-1.5 rounded-full border bg-card px-2.5 text-xs"><Icon icon={ic.file} size={12} class="text-muted-foreground" />{c.name}</span>}
                          </For>
                          <Show when={t() >= s.until}><span class="m-pop inline-flex h-6 items-center text-xs text-muted-foreground">+3 more</span></Show>
                        </div>
                      </Show>
                      <Show when={s.subs}>
                        <ul class="mt-1.5 flex flex-col gap-1 border-l pl-3">
                          <For each={s.subs!.filter((u) => t() >= u.at)}>{(u) => <li class="m-rise flex items-center gap-2 text-muted-foreground/80"><span class="size-1 rounded-full bg-muted-foreground/60" />{u.label}</li>}</For>
                        </ul>
                      </Show>
                    </div>
                  </li>
                );
              }}
            </For>
          </ol>
        </div>
      </div>
      <Show when={props.withQuestion && t() > THINK_END}>
        <Card class="m-rise gap-3 rounded-xl px-5 py-4 shadow-none">
          <p class="flex items-center gap-2 text-xs text-muted-foreground"><span class="m-ring size-1.5 rounded-full bg-signal" />Question · Worker L1</p>
          <AskCard />
        </Card>
      </Show>
    </div>
  );
}

export default function Thinking() {
  return (
    <div data-dir="gen" class="h-full w-full p-7">
      <ThinkingCard withQuestion />
    </div>
  );
}
