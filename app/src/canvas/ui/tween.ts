// A number that eases toward a target in `ms`. Timer based, so it also runs in a
// background tab; the old value is replaced at once if the target moves again.

import { createSignal, onCleanup } from "solid-js";

export function createTween(initial: number, ms = 900) {
  const [v, setV] = createSignal(initial);
  let id = 0;
  onCleanup(() => clearInterval(id));
  const to = (target: number) => {
    clearInterval(id);
    const from = v();
    const start = performance.now();
    id = window.setInterval(() => {
      const p = Math.min(1, (performance.now() - start) / ms);
      setV(from + (target - from) * (1 - Math.pow(1 - p, 3)));
      if (p === 1) clearInterval(id);
    }, 24);
  };
  return [v, to] as const;
}
