// Run progress as one line of text: `plan --- bar ·:=+· work ··· review`.
// Done steps are plain, the current label decodes and its wire flows.

import { For, Show } from "solid-js";
import Ascii from "./Ascii";
import { decode, wire } from "./pieces";

export const STEPS = [
  { phase: "planning", label: "plan" },
  { phase: "calibrating", label: "bar" },
  { phase: "working", label: "work" },
  { phase: "reviewing", label: "review" },
  { phase: "gating", label: "gate" },
  { phase: "done", label: "ready" },
] as const;

const order = (p: string) => (p === "fixing" ? 2 : STEPS.findIndex((s) => s.phase === p));

export default function RunTrack(props: { phase: string }) {
  const cur = () => (props.phase === "done" ? STEPS.length : order(props.phase));
  const label = (i: number) => (i === 2 && props.phase === "fixing" ? "fixing" : STEPS[i].label);
  return (
    <p class="run-track" aria-label={`Run : ${props.phase}`}>
      <For each={STEPS}>
        {(_, i) => (
          <>
            <Show when={i() > 0}>
              <Ascii class={`rt-wire ${i() < cur() ? "done" : i() === cur() ? "now" : ""}`} frame={wire(3, i() < cur() ? "done" : i() === cur() ? "now" : "todo")} fps={14} />
            </Show>
            <Show when={i() === cur()} fallback={<span class="rt-step" classList={{ done: i() < cur() }}>{label(i())}</span>}>
              <Show when={label(i())} keyed>
                {(l) => <Ascii class="rt-step now" frame={decode(l)} fps={14} />}
              </Show>
            </Show>
          </>
        )}
      </For>
    </p>
  );
}
