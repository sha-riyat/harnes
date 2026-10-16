// One requestAnimationFrame loop shared by every ASCII animation.
// Each subscriber has its own frame rate and play time. Time only moves
// while the element is on screen and the window is visible; with reduced
// motion, a subscriber is drawn once at its still time and never ticks.

type Sub = { fps: number; last: number; t: number; visible: boolean; tick: (t: number) => void };

const subs = new Set<Sub>();
let raf = 0;

export const reducedMotion = () => typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;

function loop(now: number) {
  raf = 0;
  for (const s of subs) {
    if (!s.visible) {
      s.last = now;
      continue;
    }
    const dt = now - s.last;
    if (dt < 1000 / s.fps - 2) continue;
    s.last = now;
    s.t += Math.min(dt, 100) / 1000;
    s.tick(s.t);
  }
  start();
}

function start() {
  if (!raf && subs.size && !document.hidden) raf = requestAnimationFrame(loop);
}

document.addEventListener("visibilitychange", () => {
  const now = performance.now();
  subs.forEach((s) => (s.last = now));
  start();
});

export interface Animation {
  /** Draw again at the current time (after a prop change). */
  redraw(): void;
  stop(): void;
}

/** Ticks `tick(t)` at `fps` while `el` is visible. `still` is the time shown with reduced motion. */
export function animate(el: Element, fps: number, tick: (t: number) => void, still = 0): Animation {
  if (reducedMotion()) {
    tick(still);
    return { redraw: () => tick(still), stop: () => {} };
  }
  const sub: Sub = { fps, last: performance.now(), t: 0, visible: true, tick };
  const io = new IntersectionObserver(([e]) => (sub.visible = e.isIntersecting));
  io.observe(el);
  subs.add(sub);
  tick(0);
  start();
  return {
    redraw: () => tick(sub.t),
    stop: () => {
      io.disconnect();
      subs.delete(sub);
    },
  };
}
