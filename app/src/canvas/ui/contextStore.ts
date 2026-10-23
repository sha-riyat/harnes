// The context window of the agent in the chat, shared by the ring in the message box
// and the panel it opens, so compacting in one is seen in the other.

import { createRoot, createSignal } from "solid-js";
import { contextWindow as cw } from "../data";
import { createTween } from "./tween";

export type Phase = "idle" | "running" | "done";

export const ctx = createRoot(() => {
  const original = cw.parts.find((p) => p.id === "messages")!.k;
  const [messages, to] = createTween(original, 1400);
  const [phase, setPhase] = createSignal<Phase>("idle");
  let timers: number[] = [];

  const parts = () => cw.parts.map((p) => (p.id === "messages" ? { ...p, k: messages() } : p));
  const used = () => parts().reduce((s, p) => s + p.k, 0);

  const reset = () => {
    timers.forEach(clearTimeout);
    timers = [];
    setPhase("idle");
    to(original);
  };
  const compact = () => {
    if (phase() !== "idle") return;
    setPhase("running");
    to(cw.after);
    timers.push(window.setTimeout(() => setPhase("done"), 1500), window.setTimeout(reset, 8000));
  };

  return {
    phase,
    parts,
    used,
    share: () => used() / cw.total,
    free: () => cw.total - used() - cw.reserved,
    freed: () => original - cw.after,
    compact,
    reset,
  };
});
