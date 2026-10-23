// A small clock that replays what the crew does, so the motion can be judged
// without a real run. One integer tick drives every panel; each panel maps the
// tick to its own state, so nothing here knows about the panels.

import { createSignal, onCleanup } from "solid-js";

export const LAST = 20;
const HOLD = 5;

export function createSim(ms = 1400, last = LAST, hold = HOLD) {
  const [t, setT] = createSignal(0);
  const [paused, setPaused] = createSignal(false);
  const id = window.setInterval(() => {
    if (document.hidden || paused()) return;
    setT((v) => (v >= last + hold ? 0 : v + 1));
  }, ms);
  onCleanup(() => clearInterval(id));
  return { t, paused, replay: () => setT(0), toggle: () => setPaused((p) => !p) };
}

export type Sim = ReturnType<typeof createSim>;

// Shared moments of the replay.
export const MERGED_AT = 13;
export const merged = (t: number) => t >= MERGED_AT;
