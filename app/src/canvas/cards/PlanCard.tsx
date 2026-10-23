// The plan: lots in waves, and which lot waits for which. Nodes rise in turn, the
// edges draw themselves, and an edge lights up when what it waits for is done.
// When L2 is done, L3 is released and starts.

import { For, Show } from "solid-js";
import { Card } from "@/components/ui/card";
import { cx } from "@/lib/cva";
import Icon from "../../ui/Icon";
import { agents, lotLabel, lots, type LotStatus } from "../data";
import { ic } from "../icons";
import AgentIcon from "../ui/AgentIcon";
import { useFlow } from "../ui/flow";
import { createSim } from "../ui/sim";
import { deps, waves } from "./cardData";

const NODE_H = 54;
const GAP = 12;
const COL_W = 252;
const COL_GAP = 84;
const WIDTH = 16 + COL_W * 2 + COL_GAP + 16; // the card is exactly as wide as its two columns
const TOP = 40;
const rowY = (i: number) => TOP + i * (NODE_H + GAP);
const colX = (c: number) => c * (COL_W + COL_GAP);

// The replay: L2 finishes at tick 9, which releases L3 at tick 10.
const status = (id: string, t: number, answered = false): LotStatus => {
  if (id === "L1" && answered) return "working";
  if (id === "L2") return t >= 9 ? "done" : "working";
  if (id === "L3") return t >= 11 ? "working" : "planned";
  if (id === "L5") return t >= 7 ? "done" : "review";
  return lots.find((l) => l.id === id)!.status;
};

export default function PlanCard() {
  const flow = useFlow();
  const sim = createSim(1000, 14, 4);
  const t = sim.t;
  const pos = (id: string) => {
    const c = waves.findIndex((w) => w.lots.includes(id));
    return { c, r: waves[c].lots.indexOf(id) };
  };
  const H = rowY(3) + NODE_H + 12;
  return (
    <Card class="gap-0 rounded-xl py-0 shadow-none" style={{ width: `${WIDTH}px`, "max-width": "100%" }}>
      <header class="flex items-center gap-2.5 border-b px-4 py-3">
        <AgentIcon role="planner" state="done" />
        <div class="flex-1 leading-tight"><p class="text-sm font-medium">Plan · 6 lots in 2 waves</p><p class="text-xs text-muted-foreground">Planner · Opus 5.5</p></div>
        <span class="num text-xs text-muted-foreground">{lots.filter((l) => status(l.id, t(), flow.need() === 0) === "done").length} of 6 done</span>
      </header>
      <div class="relative px-4 pb-4" style={{ height: `${H + 8}px` }}>
        <For each={waves}>{(w, c) => <p class="absolute text-xs text-muted-foreground" style={{ left: `${16 + colX(c())}px`, top: "12px" }}>Wave {w.n}</p>}</For>
        <svg class="pointer-events-none absolute inset-x-4 top-0" width="100%" height={H} aria-hidden="true">
          <For each={deps}>
            {([from, to]) => {
              const a = pos(from), b = pos(to);
              const x1 = colX(a.c) + COL_W, y1 = rowY(a.r) + NODE_H / 2, x2 = colX(b.c), y2 = rowY(b.r) + NODE_H / 2;
              const lit = () => status(from, t()) === "done";
              return (
                <Show when={t() >= 2}>
                  <path d={`M${x1} ${y1} C ${x1 + 40} ${y1}, ${x2 - 40} ${y2}, ${x2} ${y2}`} fill="none" stroke="currentColor" stroke-width="1.5" class={cx("m-draw m-edge text-foreground", lit() ? "opacity-90" : "opacity-25")} style={{ "--len": "260", "--dur": "500ms" }} />
                </Show>
              );
            }}
          </For>
        </svg>
        <For each={lots}>
          {(l, i) => {
            const p = pos(l.id);
            const s = () => status(l.id, t(), flow.need() === 0);
            const owner = () => agents.find((a) => a.key === l.agent);
            return (
              <div
                class={cx("m-rise absolute flex items-center gap-2.5 rounded-lg border bg-card px-3 transition-[border-color,opacity] duration-300", s() === "planned" && "opacity-60", s() === "needs" && "border-signal/60", s() === "working" && "border-foreground/40")}
                style={{ left: `${16 + colX(p.c)}px`, top: `${rowY(p.r)}px`, width: `${COL_W}px`, height: `${NODE_H}px`, "--i": i() }}
              >
                <span class="grid size-7 shrink-0 place-items-center">
                  <Show when={owner()} fallback={<Icon icon={ic.cube} size={16} class="text-muted-foreground" />}>
                    <AgentIcon role={owner()!.role} state={s() === "done" ? "done" : s() === "working" ? "working" : s() === "needs" ? "blocked" : "idle"} />
                  </Show>
                </span>
                <div class="min-w-0 flex-1 leading-tight">
                  <p class="truncate text-sm font-medium"><span class="num text-muted-foreground">{l.id}</span> {l.title}</p>
                  <p class="text-xs text-muted-foreground">{lotLabel[s()]} · {l.files} {l.files === 1 ? "file" : "files"}</p>
                </div>
              </div>
            );
          }}
        </For>
      </div>
    </Card>
  );
}
