// Small ASCII animations. One shared clock drives them all; it stops when the
// window is hidden or the user prefers reduced motion, so idle cost is ~zero.

import spinners from "cli-spinners";
import { For, createMemo, createSignal, onCleanup, type JSX } from "solid-js";

const reduced = typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;
const [tick, setTick] = createSignal(0);
let users = 0;
let timer: number | undefined;

function startClock() {
  if (reduced || timer !== undefined) return;
  timer = window.setInterval(() => {
    if (!document.hidden) setTick((t) => t + 1);
  }, 80);
}

function useClock(): () => number {
  users++;
  startClock();
  onCleanup(() => {
    if (--users === 0 && timer !== undefined) {
      clearInterval(timer);
      timer = undefined;
    }
  });
  return tick;
}

/** cli-spinners frame, advancing at the spinner's own pace. */
export function Spinner(props: { name?: keyof typeof spinners; class?: string }) {
  const t = useClock();
  const sp = () => spinners[props.name ?? "dots"];
  const frame = () => {
    const s = sp();
    return s.frames[Math.floor((t() * 80) / s.interval) % s.frames.length];
  };
  return <span class={`spinner ${props.class ?? ""}`} aria-hidden="true">{frame()}</span>;
}

/** A message travelling from an author to a thread: `worker-L1 ··●··▸ lot-L1`. */
export function FlowLine(props: { from: string; to: string; born: number }) {
  const t = useClock();
  const WIDTH = 14;
  const track = createMemo(() => {
    const age = Date.now() - props.born + t() * 0; // re-evaluated on each tick
    const pos = Math.min(WIDTH - 1, Math.floor(age / 90));
    const done = age > WIDTH * 90 + 400;
    return Array.from({ length: WIDTH }, (_, i) => (done ? (i === WIDTH - 1 ? "▸" : "─") : i === pos ? "●" : i < pos ? "·" : " ")).join("");
  });
  return (
    <span class="flow mono">
      <span class="flow-from">{props.from}</span>
      <span class="flow-track">{track()}</span>
      <span class="flow-to">{props.to}</span>
    </span>
  );
}

const GLYPHS = " .·:∙•°";

/** Slow drifting glyph field, for empty areas. */
export function AsciiField(props: { cols?: number; rows?: number; style?: JSX.CSSProperties }) {
  const t = useClock();
  const cols = props.cols ?? 64;
  const rows = props.rows ?? 10;
  const lines = createMemo(() => {
    const phase = t() / 12;
    return Array.from({ length: rows }, (_, y) =>
      Array.from({ length: cols }, (_, x) => {
        const v = Math.sin(x * 0.35 + phase) * Math.cos(y * 0.6 - phase * 0.7) + Math.sin((x + y) * 0.12 + phase * 0.5);
        const i = Math.max(0, Math.min(GLYPHS.length - 1, Math.floor(((v + 2) / 4) * GLYPHS.length)));
        return GLYPHS[i];
      }).join(""),
    );
  });
  return (
    <pre class="ascii-field" aria-hidden="true" style={props.style}>
      <For each={lines()}>{(l) => <div>{l}</div>}</For>
    </pre>
  );
}
