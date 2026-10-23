// The context window of an agent, one click from the message box (after the
// "Context window" reference). A segmented bar of what fills it, a legend that dims
// the rest on hover, and one action: compact. Compacting plays out: the messages
// segment shrinks, the figures count down, a line says what was freed.

import { For, Show, createSignal } from "solid-js";
import { Button } from "@/components/ui/button";
import { Progress, ProgressGroup, ProgressLabel, ProgressValueLabel } from "@/components/ui/progress";
import { cx } from "@/lib/cva";
import Icon from "../../ui/Icon";
import { contextWindow as cw, minutes, usage } from "../data";
import { ic } from "../icons";
import { ctx } from "./contextStore";

const tone: Record<string, string> = { system: "bg-foreground/70", mcp: "bg-foreground/45", skills: "bg-foreground/25", messages: "bg-foreground", free: "bg-muted-foreground/15", reserved: "bg-muted-foreground/30" };
const k = (n: number) => `${n.toFixed(1)}k`;

/** The panel itself, without a frame: it sits inside a card or a popover. */
export default function ContextPanel() {
  const [hover, setHover] = createSignal<string | null>(null);
  const segments = () => [...ctx.parts().map((p) => ({ id: p.id, k: p.k })), { id: "free", k: ctx.free() }, { id: "reserved", k: cw.reserved }];
  const rows = () => [...ctx.parts(), { id: "free", label: "Free", k: ctx.free(), tip: "" }, { id: "reserved", label: "Reserved", k: cw.reserved, tip: "Kept free so compacting has room to run" }];
  const detail = () => {
    const p = rows().find((x) => x.id === hover());
    return p ? `${p.label} · ${k(p.k)} · ${((p.k / cw.total) * 100).toFixed(0)}%${p.tip ? ` · ${p.tip}` : ""}` : "";
  };
  return (
    <div class="flex w-[420px] flex-col gap-4 px-5 py-4">
      <div class="flex items-baseline justify-between text-sm">
        <p><span class="font-medium">Context</span> <span class="num text-muted-foreground">{Math.round(ctx.share() * 100)}%</span></p>
        <p class="num text-muted-foreground">{k(ctx.used())} of 1M</p>
      </div>
      <div class="flex h-2 gap-px overflow-hidden rounded-full" role="img" aria-label={`Context window, ${k(ctx.used())} used of 1M`}>
        <For each={segments()}>{(s) => <div class={cx("h-full shrink-0 transition-opacity duration-150", tone[s.id], hover() && hover() !== s.id && "opacity-25")} style={{ width: `${(s.k / cw.total) * 100}%` }} />}</For>
      </div>
      <ul class="grid grid-cols-2 gap-x-6 gap-y-1.5 text-sm">
        <For each={rows()}>
          {(p) => (
            <li class={cx("flex items-center gap-2 rounded px-1 py-0.5 transition-opacity", hover() && hover() !== p.id && "opacity-40")} onMouseEnter={() => setHover(p.id)} onMouseLeave={() => setHover(null)}>
              <span class={cx("size-2 rounded-[2px]", tone[p.id])} />
              <span class="text-muted-foreground">{p.label}</span>
              <span class="num ml-auto font-medium">{k(p.k)}</span>
            </li>
          )}
        </For>
      </ul>
      <p class="num min-h-4 text-xs text-muted-foreground"><Show when={detail()} keyed>{(d) => <span class="m-tick">{d}</span>}</Show></p>
      <div class="flex flex-col gap-3 border-t pt-3">
        <Progress value={usage.fiveHour * 100} class="gap-1.5"><ProgressGroup><ProgressLabel class="text-xs font-normal text-muted-foreground">5-hour window · resets in {minutes(usage.resetsIn)}</ProgressLabel><ProgressValueLabel class="num text-xs" /></ProgressGroup></Progress>
        <Progress value={usage.week * 100} class="gap-1.5"><ProgressGroup><ProgressLabel class="text-xs font-normal text-muted-foreground">Week · resets in {usage.weekResetsInDays} d</ProgressLabel><ProgressValueLabel class="num text-xs" /></ProgressGroup></Progress>
      </div>
      <div class="flex h-8 items-center justify-between border-t pt-3 text-sm">
        <p class="num text-muted-foreground" aria-live="polite">
          <Show when={ctx.phase() === "idle"}>{Math.round(ctx.free())}k until auto-compact</Show>
          <Show when={ctx.phase() === "running"}><span class="m-breathe">Summarizing messages</span></Show>
          <Show when={ctx.phase() === "done"}><span class="m-rise inline-flex items-center gap-1.5 text-foreground"><Icon icon={ic.checkBold} size={13} />Freed {k(ctx.freed())} tokens</span></Show>
        </p>
        <div class="flex items-center gap-1">
          <Show when={ctx.phase() === "done"}><Button variant="ghost" size="icon" class="size-8 text-muted-foreground" aria-label="Replay" title="Replay" onClick={ctx.reset}><Icon icon={ic.replay} size={15} /></Button></Show>
          <Button variant="secondary" size="sm" disabled={ctx.phase() !== "idle"} onClick={ctx.compact}>{ctx.phase() === "done" ? "Compacted" : "Compact now"}</Button>
        </div>
      </div>
    </div>
  );
}

