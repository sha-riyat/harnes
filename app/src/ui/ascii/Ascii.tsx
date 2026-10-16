// Inline ASCII animation: a span whose text is a piece's frame at play time.

import { createEffect, onCleanup, onMount } from "solid-js";
import { animate, type Animation } from "./clock";
import { mark } from "./pieces";

export default function Ascii(props: { frame: (t: number) => string; fps?: number; still?: number; class?: string; label?: string }) {
  let el!: HTMLSpanElement;
  let anim: Animation | undefined;
  const draw = (t: number) => {
    const s = props.frame(t);
    if (el.textContent !== s) el.textContent = s;
  };
  onMount(() => {
    anim = animate(el, props.fps ?? 12, draw, props.still ?? 10);
    onCleanup(() => anim?.stop());
  });
  // A new piece (state change) draws at once, even when time stands still.
  createEffect(() => {
    props.frame;
    anim?.redraw();
  });
  return (
    <span
      ref={el}
      class={`ascii ${props.class ?? ""}`}
      role={props.label ? "img" : undefined}
      aria-label={props.label}
      aria-hidden={props.label ? undefined : true}
    />
  );
}

const seedOf = (s: string) => [...s].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 997, 7);

/** An agent's state as one living cell. Agents breathe out of phase. */
export function StateMark(props: { state?: string; id?: string }) {
  return <Ascii class={`mark s-${props.state ?? "idle"}`} frame={mark(props.state ?? "idle", seedOf(props.id ?? ""))} label={props.state} fps={10} />;
}
