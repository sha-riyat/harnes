// A command that needs permission. Allowed, the buttons fold away and the
// command's output is written below it, line by line, ending on a check.

import { For, Show, createEffect, createSignal, onCleanup } from "solid-js";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { cx } from "@/lib/cva";
import Icon from "../../ui/Icon";
import { permission } from "../data";
import { ic } from "../icons";
import AgentIcon from "../ui/AgentIcon";
import { useFlow } from "../ui/flow";
import { createSim } from "../ui/sim";

const OUTPUT = ["added 1 package, and audited 214 packages in 2s", "found 0 vulnerabilities", "nodemailer 6.9.14 installed"];

export default function PermissionCard(props: { demo?: boolean; class?: string }) {
  const flow = useFlow();
  const [lines, setLines] = createSignal(0);
  const timers: number[] = [];
  onCleanup(() => timers.forEach(clearTimeout));
  const granted = () => flow.permit() !== null && flow.permit() !== "deny";

  // Output starts a beat after the command is allowed, one line every 650 ms.
  createEffect(() => {
    timers.forEach(clearTimeout);
    setLines(0);
    if (!granted()) return;
    OUTPUT.forEach((_, i) => timers.push(window.setTimeout(() => setLines(i + 1), 500 + i * 650)));
  });

  if (props.demo) {
    const sim = createSim(1000, 9, 3);
    createEffect(() => {
      if (sim.t() === 0) flow.reset();
      if (sim.t() === 3) flow.allow("once");
    });
  }

  return (
    <Card class={cx("m-rise gap-0 rounded-xl py-0 shadow-none", props.class)}>
      <header class="flex items-center gap-2.5 px-4 py-3">
        <AgentIcon role="worker" state={flow.permit() ? "working" : "blocked"} />
        <div class="min-w-0 flex-1 leading-tight">
          <p class="text-sm font-medium">{permission.agent} <span class="font-normal text-muted-foreground">wants to run a command</span></p>
          <p class="num text-xs text-muted-foreground">{permission.since} · {permission.tool}</p>
        </div>
        <Show when={flow.permit()} fallback={<span class="inline-flex items-center gap-1.5 text-xs text-muted-foreground"><Icon icon={ic.shield} size={13} />asks first</span>}>
          <span class="m-pop inline-flex items-center gap-1 text-xs text-muted-foreground"><Icon icon={ic.checkBold} size={12} />{flow.permit() === "once" ? "Allowed once" : flow.permit() === "always" ? "Always allowed" : "Denied"}</span>
        </Show>
      </header>
      <div class="px-4 pb-4">
        <code class="block rounded-md bg-muted px-3 py-2 font-mono text-[13px]"><span class="text-muted-foreground">$ </span>{permission.command}</code>
        <div class="m-fold" data-folded={flow.permit() !== null}>
          <div>
            <div class="flex items-center gap-2 pt-3">
              <Button size="sm" onClick={() => flow.allow("once")}>Allow once</Button>
              <Button size="sm" variant="outline" onClick={() => flow.allow("always")}>Always</Button>
              <Button size="sm" variant="ghost" class="text-muted-foreground" onClick={() => flow.allow("deny")}>Deny</Button>
            </div>
          </div>
        </div>
        <Show when={granted()}>
          <ul class="mt-3 flex flex-col gap-1 border-t pt-3 font-mono text-xs text-muted-foreground">
            <For each={OUTPUT.slice(0, lines())}>
              {(l, i) => (
                <li class="m-rise flex items-center gap-2">
                  <Show when={i() === OUTPUT.length - 1} fallback={<span class="w-3 text-center">·</span>}>
                    <svg viewBox="0 0 16 16" class="size-3 text-foreground"><path d="M3.5 8.5l3 3 6-7" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="m-draw" style={{ "--len": "16", "--dur": "220ms" }} /></svg>
                  </Show>
                  <span class={cx(i() === OUTPUT.length - 1 && "text-foreground")}>{l}</span>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </div>
    </Card>
  );
}
