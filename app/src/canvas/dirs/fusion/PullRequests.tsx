// Pull requests of the run. Checks run one after the other (a dot breathes,
// then a check draws itself); once approved, the branch curves back into main
// and the merge node lands with one ring. A merged request is static.

import { For, Match, Show, Switch } from "solid-js";
import { Badge } from "@/components/ui/badge";
import { Card } from "@/components/ui/card";
import { cx } from "@/lib/cva";
import Icon from "../../../ui/Icon";
import { checks, pulls } from "../../data";
import { ic } from "../../icons";
import { MERGED_AT } from "../../ui/sim";

type CheckState = "pending" | "running" | "passed";
// Tick at which check i starts, and at which it passes.
const START = [3, 5, 7];
const PASS = [5, 7, 10];

function CheckMark(props: { state: CheckState; name: string }) {
  return (
    <span class="flex items-center gap-1.5 text-xs text-muted-foreground">
      <span class="grid size-3.5 place-items-center">
        <Switch>
          <Match when={props.state === "pending"}><span class="size-1.5 rounded-full border border-muted-foreground/60" /></Match>
          <Match when={props.state === "running"}><span class="m-breathe size-1.5 rounded-full bg-foreground" /></Match>
          <Match when={props.state === "passed"}>
            <svg viewBox="0 0 16 16" class="size-3.5 text-foreground"><path d="M3.5 8.5l3 3 6-7" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" class="m-draw" style={{ "--len": "16", "--dur": "200ms" }} /></svg>
          </Match>
        </Switch>
      </span>
      <span class={cx(props.state === "passed" && "text-foreground")}>{props.name}</span>
    </span>
  );
}

/** The branch leaves main, runs alongside, and (once merged) curves back in. */
function Graph(props: { joined: boolean; joining: boolean; settled?: boolean }) {
  return (
    <svg viewBox="0 0 120 30" class="h-[30px] w-[120px] shrink-0 text-muted-foreground" aria-hidden="true">
      <line x1="4" y1="22" x2="116" y2="22" stroke="currentColor" stroke-opacity="0.35" stroke-width="1.5" />
      <path d="M20 22 C 30 22, 30 8, 42 8 H 78" fill="none" stroke="currentColor" stroke-width="1.5" />
      <Show when={props.joined || props.joining}>
        <path d="M78 8 C 90 8, 90 22, 100 22" fill="none" stroke="currentColor" stroke-width="1.5" class={cx(!props.settled && "m-draw")} style={{ "--len": "40", "--dur": "320ms" }} />
      </Show>
      <circle cx="20" cy="22" r="3" class="fill-background" stroke="currentColor" stroke-width="1.5" />
      <circle cx="100" cy="22" r="3" stroke="currentColor" stroke-width="1.5" class={cx(props.joined ? "fill-foreground" : "fill-background", "transition-colors duration-300")} />
    </svg>
  );
}

export default function PullRequests(props: { t: () => number }) {
  const t = props.t;
  const stateOf = (i: number): CheckState => (t() >= PASS[i] ? "passed" : t() >= START[i] ? "running" : "pending");
  const status = () => (t() >= MERGED_AT ? "Merged" : t() >= MERGED_AT - 1 ? "Merging" : t() >= PASS[2] ? "Approved" : t() >= START[0] ? "Checks running" : "Open");
  return (
    <Card class="gap-0 rounded-lg py-0 shadow-none">
      <header class="flex items-center gap-2.5 border-b px-4 py-2.5">
        <Icon icon={ic.pullRequest} size={16} class="text-muted-foreground" />
        <h2 class="text-sm font-semibold">Pull requests</h2>
        <p class="num flex-1 text-sm text-muted-foreground">{pulls.length} in this run</p>
      </header>
      <For each={pulls}>
        {(p) => {
          const done = p.state === "merged";
          const joined = () => done || t() >= MERGED_AT;
          return (
            <div class="flex items-center gap-4 border-b px-4 py-2.5 last:border-b-0">
              <Icon icon={joined() ? ic.merge : ic.pullRequest} size={17} class={cx("shrink-0", joined() ? "text-foreground" : "text-muted-foreground")} />
              <div class="w-[190px] min-w-0">
                <p class="truncate text-sm font-medium"><span class="num text-muted-foreground">#{p.n}</span> {p.lot} · {p.title}</p>
                <p class="flex items-center gap-1 truncate font-mono text-xs text-muted-foreground"><Icon icon={ic.branch} size={12} />{p.branch}</p>
              </div>
              <div class="relative">
                <Graph joined={joined()} joining={!done && t() >= MERGED_AT - 1} settled={done} />
                <Show when={!done && t() === MERGED_AT}><span class="m-land absolute top-[19px] left-[97px] size-[6px] rounded-full text-foreground" /></Show>
              </div>
              <div class="flex flex-1 items-center gap-4">
                <For each={checks}>{(c, i) => <CheckMark name={c} state={done ? "passed" : stateOf(i())} />}</For>
              </div>
              <Badge variant={done || t() >= MERGED_AT ? "default" : "outline"} class="num w-[110px] justify-center font-normal transition-colors duration-300">{done ? "Merged" : status()}</Badge>
              <span class="num w-10 text-right text-xs text-muted-foreground">{p.at}</span>
            </div>
          );
        }}
      </For>
    </Card>
  );
}
