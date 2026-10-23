// A typed question from an agent. Open, it waits with one ring on its dot. Once
// the user answers, the body folds away and one line remains; the agent's tile
// turns back to "working".

import { For, Show, createEffect, createSignal } from "solid-js";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { RadioGroup, RadioGroupItem, RadioGroupItemControl, RadioGroupItemIndicator, RadioGroupItemInput, RadioGroupItemLabel, RadioGroupItems } from "@/components/ui/radio-group";
import { cx } from "@/lib/cva";
import Icon from "../../ui/Icon";
import { agents, question, waited } from "../data";
import { ic } from "../icons";
import AgentIcon from "../ui/AgentIcon";
import { useFlow } from "../ui/flow";
import { createSim } from "../ui/sim";

const who = agents.find((a) => a.key === "w1")!;

export default function QuestionCard(props: { demo?: boolean; class?: string }) {
  const flow = useFlow();
  const [pick, setPick] = createSignal("smtp");
  const answered = () => flow.choice() !== null;
  const label = () => question.options.find((o) => o.id === flow.choice())?.label;

  if (props.demo) {
    const sim = createSim(1000, 9, 3);
    createEffect(() => {
      const t = sim.t();
      if (t === 0) flow.reset();
      if (t === 2) setPick("api");
      if (t === 4) setPick("smtp");
      if (t === 6) flow.answer(pick());
    });
  }

  return (
    <Card class={cx("m-rise gap-0 rounded-xl py-0 shadow-none", props.class)}>
      <header class="flex items-center gap-2.5 px-4 py-3">
        <AgentIcon role="worker" state={answered() ? "working" : "blocked"} />
        <div class="min-w-0 flex-1 leading-tight">
          <p class="text-sm font-medium">{who.name} <span class="font-normal text-muted-foreground">asks</span></p>
          <p class="num text-xs text-muted-foreground">{question.since} · lot L1</p>
        </div>
        <Show when={!answered()} fallback={<span class="m-pop inline-flex items-center gap-1 text-xs text-muted-foreground"><Icon icon={ic.checkBold} size={12} />Answered</span>}>
          <span class="inline-flex items-center gap-1.5 text-xs text-muted-foreground"><span class="m-ring size-1.5 rounded-full bg-signal" />waiting {waited} min</span>
        </Show>
      </header>
      <div class="m-fold" data-folded={answered()}>
        <div>
          <div class="flex flex-col gap-3 px-4 pb-4">
            <p class="text-sm leading-snug font-medium">{question.text}</p>
            <RadioGroup value={pick()} onChange={setPick} aria-label="Answer">
              <RadioGroupItems class="grid grid-cols-2 gap-2">
                <For each={question.options}>
                  {(o) => (
                    <RadioGroupItem value={o.id} class="rounded-lg border px-3 py-2.5 transition-[border-color,background-color] duration-150 hover:bg-accent data-[checked]:border-foreground">
                      <RadioGroupItemInput />
                      <RadioGroupItemControl><RadioGroupItemIndicator /></RadioGroupItemControl>
                      <RadioGroupItemLabel class="flex flex-col gap-0 leading-tight">{o.label}<span class="text-xs font-normal text-muted-foreground">{o.detail}</span></RadioGroupItemLabel>
                    </RadioGroupItem>
                  )}
                </For>
              </RadioGroupItems>
            </RadioGroup>
            <div class="flex items-center gap-2">
              <Button size="sm" onClick={() => flow.answer(pick())}>Send answer</Button>
              <Button size="sm" variant="ghost" class="text-muted-foreground">Skip</Button>
              <span class="ml-auto text-xs text-muted-foreground">Default: SMTP in 5 min</span>
            </div>
          </div>
        </div>
      </div>
      <Show when={answered()}>
        <p class="m-rise border-t px-4 py-2.5 text-sm text-muted-foreground">You chose <span class="font-medium text-foreground">{label()}</span> · Worker L1 carries on.</p>
      </Show>
    </Card>
  );
}
