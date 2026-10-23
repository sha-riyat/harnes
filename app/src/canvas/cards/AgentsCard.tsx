// The crew as icons. One tile per agent: its role icon, its name, its model, and
// the line it is on right now. A working agent breathes a ring; the one waiting
// for the user wears the one hue; a finished one carries a check.

import { For, Show } from "solid-js";
import { Card } from "@/components/ui/card";
import { cx } from "@/lib/cva";
import { agents, stateLabel, workLog } from "../data";
import AgentIcon, { roleLabel } from "../ui/AgentIcon";
import { useFlow } from "../ui/flow";
import { createSim } from "../ui/sim";

export default function AgentsCard(props: { class?: string }) {
  const sim = createSim(2200, 1000, 1);
  const flow = useFlow();
  // Worker L2 moves through its log; the others keep their line.
  const task = (key: string, fallback: string) => (key === "w2" ? workLog[sim.t() % workLog.length].label : fallback);
  return (
    <Card class={cx("gap-0 rounded-xl py-0 shadow-none", props.class)}>
      <header class="flex items-center border-b px-4 py-3"><p class="text-sm font-medium">Crew <span class="font-normal text-muted-foreground">· {agents.length} agents</span></p></header>
      <ul class="grid grid-cols-2 gap-px bg-border [&>li:last-child:nth-child(odd)]:col-span-2">
        <For each={agents}>
          {(a, i) => {
            const state = () => flow.stateOf(a);
            return (
              <li class="m-rise flex gap-3 bg-card px-4 py-3.5 transition-colors duration-150 hover:bg-accent/40" style={{ "--i": i() }}>
                <AgentIcon role={a.role} state={state()} size="lg" />
                <div class="min-w-0 flex-1 leading-tight">
                  <p class="flex items-center gap-2 text-sm font-medium">{a.name}<span class="text-xs font-normal text-muted-foreground">{a.model}</span></p>
                  <p class="text-xs text-muted-foreground">{roleLabel[a.role]}</p>
                  <p class={cx("mt-1.5 truncate text-xs", state() === "blocked" ? "text-foreground" : "text-muted-foreground")}>
                    <Show when={a.key === "w2"} fallback={<>{state() === "working" && a.key === "w1" ? "Carries on with SMTP" : a.task}</>}>
                      <Show when={task(a.key, a.task)} keyed>{(l) => <span class="m-tick">{l}</span>}</Show>
                    </Show>
                  </p>
                  <p class="num mt-0.5 text-[11px] text-muted-foreground/80">{stateLabel[state()]} · since {a.since}</p>
                </div>
              </li>
            );
          }}
        </For>
      </ul>
    </Card>
  );
}
