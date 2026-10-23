// The end of a run: "ready for you". While the Orchestrator writes its summary the
// card breathes; the summary is streamed, the figures count up one after the other,
// and the next actions rise last. Nothing here asks for a decision before it is ready.

import { For, Show, createEffect } from "solid-js";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import Icon from "../../ui/Icon";
import { ic } from "../icons";
import AgentIcon from "../ui/AgentIcon";
import { createSim } from "../ui/sim";
import { createTypewriter } from "../ui/typewriter";
import { gate } from "./cardData";

export default function GateCard() {
  const sim = createSim(1000, 12, 5);
  const tw = createTypewriter(gate.summary, 70);
  createEffect(() => {
    if (sim.t() === 0) tw.restart(1200);
  });
  const t = sim.t;
  const ready = () => tw.done();
  return (
    <Card class="gap-0 rounded-xl py-0 shadow-none">
      <header class="flex items-center gap-2.5 px-4 py-3">
        <AgentIcon role="orchestrator" state={ready() ? "done" : "working"} />
        <div class="flex-1 leading-tight">
          <p class="text-sm font-medium">{ready() ? "Ready for you" : "Wrapping up"}</p>
          <p class="text-xs text-muted-foreground">Orchestrator · run on harness-test</p>
        </div>
        <Show when={ready()} fallback={<span class="m-breathe text-xs text-muted-foreground">writing the summary</span>}>
          <span class="m-pop inline-flex items-center gap-1 text-xs text-muted-foreground"><Icon icon={ic.checkBold} size={12} />38 min</span>
        </Show>
      </header>
      <p class={`min-h-[66px] px-4 pb-3 text-sm leading-relaxed ${ready() ? "" : "m-caret"}`}>{tw.text()}</p>
      <div class="grid grid-cols-4 divide-x border-t">
        <For each={gate.stats}>
          {(s, i) => (
            <div class="flex flex-col gap-0.5 px-4 py-3">
              <p class="text-xs text-muted-foreground">{s.label}</p>
              <Show when={ready() && t() >= 0} fallback={<p class="h-7 text-xl text-muted-foreground/40">–</p>}>
                <p class="num flex h-7 items-baseline gap-1.5 text-xl leading-7 tracking-tight"><span class="m-tick" style={{ "animation-delay": `${i() * 120}ms` }}>{s.value}</span>{s.unit && <span class="m-tick text-xs text-muted-foreground" style={{ "animation-delay": `${i() * 120 + 60}ms` }}>{s.unit}</span>}</p>
              </Show>
            </div>
          )}
        </For>
      </div>
      <Show when={ready()}>
        <footer class="m-rise flex items-center gap-2 border-t px-4 py-3" style={{ "animation-delay": "500ms" }}>
          <Button size="sm"><Icon icon={ic.pullRequest} size={15} />Open the pull request</Button>
          <Button size="sm" variant="outline">Review the diff</Button>
          <Button size="sm" variant="ghost" class="ml-auto text-muted-foreground">Start another run</Button>
        </footer>
      </Show>
    </Card>
  );
}
