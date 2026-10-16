import { For, Show } from "solid-js";
import { crew } from "../store";
import Icon from "../ui/Icon";
import * as I from "../ui/icons";
import { setPanelOpen, setPanelTab } from "../ui/state";
import { isRunning } from "./Composer";

const STEPS = [
  { phase: "planning", label: "Plan" },
  { phase: "calibrating", label: "Barre" },
  { phase: "working", label: "Travail" },
  { phase: "reviewing", label: "Revue" },
  { phase: "gating", label: "Porte" },
  { phase: "done", label: "Prêt" },
] as const;

const order = (p: string) => {
  if (p === "fixing") return 2; // corrections = back to work
  return STEPS.findIndex((s) => s.phase === p);
};

/** Pinned crew progress, always visible above the workspace while a run is active. */
export default function Progress() {
  const current = () => order(crew.phase);
  const last = () => crew.log[crew.log.length - 1];
  return (
    <Show when={crew.phase !== "idle"}>
      <div class="progress" classList={{ failed: crew.phase === "failed" || crew.phase === "cancelled" }}>
        <ol>
          <For each={STEPS}>
            {(s, i) => {
              const state = () => (crew.phase === "done" || i() < current() ? "done" : i() === current() ? "now" : "todo");
              return (
                <li class={state()}>
                  <Show when={state() === "done"} fallback={<Show when={state() === "now" && isRunning()} fallback={<span class="pip" />}><Icon icon={I.Loading} size={13} class="spin" /></Show>}>
                    <Icon icon={I.Tick} size={13} stroke={2.2} />
                  </Show>
                  {s.label}
                  <Show when={s.phase === "working" && crew.phase === "fixing"}>
                    <span class="badge warn">corrections</span>
                  </Show>
                </li>
              );
            }}
          </For>
        </ol>
        <button class="progress-msg" title="Voir le journal" onClick={() => { setPanelTab("crew"); setPanelOpen(true); }}>
          <span>{last()?.message}</span>
          <Icon icon={I.ArrowRight} size={14} />
        </button>
      </div>
    </Show>
  );
}
