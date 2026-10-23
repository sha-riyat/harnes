// A test run. Suites start in turn (a dot breathes), pass (a check draws), and a
// bar fills at the true ratio. The summary counts up and stays.

import { For, Match, Switch } from "solid-js";
import { Card } from "@/components/ui/card";
import { cx } from "@/lib/cva";
import Icon from "../../ui/Icon";
import { ic } from "../icons";
import { createSim } from "../ui/sim";
import { suites, testsTotal } from "./cardData";

const START = [1, 3, 5]; // tick at which each suite starts; it passes two ticks later

export default function TestsCard() {
  const sim = createSim(1000, 10, 4);
  const t = sim.t;
  const passedOf = (i: number) => (t() >= START[i] + 2 ? suites[i].tests : t() >= START[i] ? Math.round(suites[i].tests / 2) : 0);
  const passed = () => suites.reduce((s, _, i) => s + passedOf(i), 0);
  const done = () => t() >= START[2] + 2;
  return (
    <Card class="gap-0 rounded-xl py-0 shadow-none">
      <header class="flex items-center gap-2.5 border-b px-4 py-3">
        <Icon icon={ic.terminal} size={17} class="text-muted-foreground" />
        <div class="flex-1 leading-tight"><p class="text-sm font-medium">npm test</p><p class="text-xs text-muted-foreground">Worker L2 · lot L2</p></div>
        <span class="num text-sm"><span class="font-medium">{passed()}</span> <span class="text-muted-foreground">/ {testsTotal}</span></span>
      </header>
      <div class="px-4 pt-3"><div class="h-1.5 overflow-hidden rounded-full bg-muted"><div class="m-fill h-full rounded-full bg-foreground" style={{ width: `${(passed() / testsTotal) * 100}%` }} /></div></div>
      <ul class="flex flex-col px-4 py-2">
        <For each={suites}>
          {(s, i) => {
            const state = () => (t() >= START[i()] + 2 ? "pass" : t() >= START[i()] ? "run" : "wait");
            return (
              <li class={cx("flex h-9 items-center gap-3 text-sm transition-opacity duration-300", state() === "wait" && "opacity-40")}>
                <span class="grid size-4 place-items-center">
                  <Switch>
                    <Match when={state() === "pass"}><svg viewBox="0 0 16 16" class="size-4"><path d="M3.5 8.5l3 3 6-7" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" class="m-draw" style={{ "--len": "16", "--dur": "200ms" }} /></svg></Match>
                    <Match when={state() === "run"}><span class="m-breathe size-1.5 rounded-full bg-foreground" /></Match>
                    <Match when={true}><span class="size-1.5 rounded-full border border-muted-foreground/60" /></Match>
                  </Switch>
                </span>
                <span class="flex-1">{s.name}</span>
                <span class="text-xs text-muted-foreground">{state() === "pass" && s.note}</span>
                <span class="num w-16 text-right text-xs text-muted-foreground">{passedOf(i())} / {s.tests}</span>
                <span class="num w-12 text-right text-xs text-muted-foreground">{state() === "pass" ? `${(s.ms / 1000).toFixed(1)}s` : "–"}</span>
              </li>
            );
          }}
        </For>
      </ul>
      <footer class="flex h-11 items-center gap-2 border-t px-4 text-sm">
        <Switch>
          <Match when={done()}><span class="m-rise inline-flex items-center gap-2 font-medium"><Icon icon={ic.checkBold} size={14} />{testsTotal} passed, 0 failed</span><span class="num ml-auto text-xs text-muted-foreground">4.2s</span></Match>
          <Match when={true}><span class="m-breathe text-muted-foreground">Running</span></Match>
        </Switch>
      </footer>
    </Card>
  );
}
