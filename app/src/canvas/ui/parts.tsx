// Small pieces shared by the directions. Grey by default; the one hue (--signal)
// marks what needs the user.

import { For } from "solid-js";
import { cx } from "@/lib/cva";
import { lotLabel, lots as planned, type LotStatus } from "../data";

export { Initials, Mark, StateDot } from "../../ui/kit/bits";

/** The run as one strip: a segment per lot, in plan order. Marks the lot that needs you. */
export function WaveStrip(props: { class?: string; override?: Record<string, LotStatus> }) {
  const tone: Record<LotStatus, string> = {
    done: "bg-foreground",
    review: "bg-foreground/60",
    working: "bg-foreground/35",
    needs: "bg-signal",
    planned: "bg-muted-foreground/20",
  };
  return (
    <div class={cx("flex gap-1", props.class)} role="img" aria-label="Lots of the run">
      <For each={planned}>
        {(l) => {
          const status = () => props.override?.[l.id] ?? l.status;
          return (
          <div class="flex-1" title={`${l.id} · ${l.title} · ${lotLabel[status()]}`}>
            <div class={cx("m-tone h-1.5 rounded-full", tone[status()])} />
            <p class="num mt-1.5 text-xs text-muted-foreground">{l.id}</p>
          </div>
          );
        }}
      </For>
    </div>
  );
}

export const money = (n: number) => n.toLocaleString("en-GB");
