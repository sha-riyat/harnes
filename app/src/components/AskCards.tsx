// Typed questions from agents (`ask_user`). The agent describes what it needs in
// one of three registered kinds (choice, confirm, text); this draws it and sends
// the answer back. Text is always shown as text, never as markup.

import { For, Show, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import { api, type AskPending } from "../api";
import { activeProject, asks, panes, setAsks } from "../store";
import Icon from "../ui/Icon";
import { icons as D } from "../ui/icons";
import { toast } from "../ui/state";

const short = (key: string) => key.replace(/^\d+-/, "").replace(/^orch-.*/, "orchestrator");

function Card(props: { ask: AskPending }) {
  const [text, setText] = createSignal("");
  const c = () => props.ask.card;
  const done = () => setAsks((l) => l.filter((a) => a.id !== props.ask.id));
  const send = async (choice?: string, free?: string) => {
    try {
      await api.askAnswer(props.ask.id, choice, free);
      done();
    } catch (e) {
      // Expired (timeout, pane closed) or refused: say it, and drop a dead card.
      toast({ title: "Answer not delivered", body: String(e), tone: "danger" });
      if (String(e).includes("expir")) done();
    }
  };
  const decline = async () => {
    await api.askDecline(props.ask.id).catch(() => {});
    done();
  };
  return (
    <div class="att-card">
      <div class="att-head">
        <Icon icon={D.question} size={16} />
        <span class="label">question · {short(props.ask.agent)}</span>
        <span class="spacer" />
        <button class="btn ghost sm" onClick={decline} title="The agent carries on with its default, or decides alone">Skip</button>
      </div>
      <p class="att-q">{c().question}</p>
      <Show when={c().kind === "choice"}>
        <ul class="opts">
          <For each={c().options}>
            {(o, i) => (
              <li>
                <button class="btn" classList={{ solid: c().default === o.id || (!c().default && i() === 0) }} onClick={() => send(o.id)}>{o.label}</button>
                <Show when={o.detail}><span class="dim">{o.detail}</span></Show>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <Show when={c().kind === "confirm"}>
        <div class="att-actions">
          <button class="btn" classList={{ solid: c().default !== "no" }} onClick={() => send("yes")}>Yes</button>
          <button class="btn" classList={{ solid: c().default === "no" }} onClick={() => send("no")}>No</button>
        </div>
      </Show>
      <Show when={c().kind === "text"}>
        <div class="att-actions">
          <input
            class="field"
            value={text()}
            maxLength={2000}
            placeholder={c().default ?? "Your answer"}
            onInput={(e) => setText(e.currentTarget.value)}
            onKeyDown={(e) => e.key === "Enter" && text().trim() && send(undefined, text())}
          />
          <button class="btn solid" disabled={!text().trim()} onClick={() => send(undefined, text())}>Send</button>
        </div>
      </Show>
      <Show when={c().default}>
        <span class="label faint">if there is no answer: {c().kind === "text" ? "the default stands" : c().default}</span>
      </Show>
    </div>
  );
}

export default function AskCards() {
  const mine = createMemo(() =>
    asks().filter((a) => {
      const pane = panes.find((p) => p.key === a.agent);
      return !pane?.project || pane.project === activeProject();
    }),
  );
  // A question can end without the UI being told (time up, pane closed).
  onMount(() => {
    const t = setInterval(async () => asks().length && setAsks(await api.asks().catch(() => asks())), 5000);
    onCleanup(() => clearInterval(t));
  });
  return (
    <For each={mine()}>{(a) => <Card ask={a} />}</For>
  );
}
