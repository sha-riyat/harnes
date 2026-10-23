// Text that is written out as an agent would stream it. Timer based, so it also
// runs in a background tab. `restart` begins again after `delay` ms.

import { createSignal, onCleanup } from "solid-js";

export function createTypewriter(text: string, cps = 60) {
  const [n, setN] = createSignal(0);
  let id = 0;
  let wait = 0;
  onCleanup(() => (clearInterval(id), clearTimeout(wait)));
  const restart = (delay = 0) => {
    clearInterval(id);
    clearTimeout(wait);
    setN(0);
    wait = window.setTimeout(() => {
      id = window.setInterval(() => {
        setN((v) => Math.min(text.length, v + 2));
        if (n() >= text.length) clearInterval(id);
      }, 1000 / (cps / 2));
    }, delay);
  };
  return { text: () => text.slice(0, n()), done: () => n() >= text.length, restart };
}
