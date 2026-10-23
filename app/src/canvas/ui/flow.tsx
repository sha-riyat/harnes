// What the user has decided so far, shared by the cards of one preview. Answering
// the question and allowing the command is what moves Worker L1 from "needs you"
// back to "working", so every place that shows the agent follows.

import { createContext, createSignal, useContext, type JSX } from "solid-js";
import { agents, type Agent, type State } from "../data";

export function createFlow() {
  const [choice, setChoice] = createSignal<string | null>(null);
  const [permit, setPermit] = createSignal<"once" | "always" | "deny" | null>(null);
  const need = () => (choice() ? 0 : 1) + (permit() ? 0 : 1);
  return {
    choice,
    permit,
    need,
    answer: (id: string) => setChoice(id),
    allow: (kind: "once" | "always" | "deny") => setPermit(kind),
    reset: () => (setChoice(null), setPermit(null)),
    /** Worker L1 waits until both decisions are made; the others keep their state. */
    stateOf: (a: Agent): State => (a.key === "w1" && need() === 0 ? "working" : a.state),
    waiting: () => agents.filter((a) => a.key === "w1" && need() > 0).length,
  };
}

export type Flow = ReturnType<typeof createFlow>;
const Ctx = createContext<Flow>();

export function FlowProvider(props: { flow: Flow; children: JSX.Element }) {
  return <Ctx.Provider value={props.flow}>{props.children}</Ctx.Provider>;
}

/** The flow of the surrounding preview, or a private one for a card on its own. */
export const useFlow = () => useContext(Ctx) ?? createFlow();
