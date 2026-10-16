// Main view: the conversation with the project's orchestrator.
// The orchestrator runs in a hidden terminal; its transcript becomes a chat.

import { For, Show, createEffect, createResource, createSignal, onCleanup, onMount } from "solid-js";
import { api, on, type ConvItem } from "../api";
import { activeProject, crew, currentProject, info, setCrew, setDrawerPane } from "../store";
import Icon from "../ui/Icon";
import Ascii from "../ui/ascii/Ascii";
import { thinking } from "../ui/ascii/pieces";
import { icons as D } from "../ui/icons";
import AsciiField from "../ui/ascii/AsciiField";
import Markdown from "../ui/Markdown";
import RunTrack from "../ui/ascii/RunTrack";
import { addProject } from "../ui/projects";
import { toast } from "../ui/state";
import Attention from "./Attention";

const RUNNING = ["planning", "calibrating", "working", "reviewing", "gating", "fixing"];
const time = (iso?: string) => (iso ? new Date(iso).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) : "");

function Message(props: { item: ConvItem }) {
  return (
    <Show
      when={props.item.role !== "tool"}
      fallback={
        <li class="msg-tool">
          <span class="mono">{props.item.tool}</span>
          <span class="dim">{props.item.text}</span>
        </li>
      }
    >
      <li class={`msg ${props.item.role}`}>
        <div class="msg-meta">
          <span class="label">{props.item.role === "user" ? "toi" : "orchestrateur"}</span>
          <span class="label faint">{time(props.item.at)}</span>
        </div>
        <Show when={props.item.role === "user"} fallback={<div class="msg-text"><Markdown text={props.item.text} /></div>}>
          <div class="msg-text plain">{props.item.text}</div>
        </Show>
      </li>
    </Show>
  );
}

export default function Conversation() {
  const [draft, setDraft] = createSignal("");
  const [pending, setPending] = createSignal<string | null>(null);
  const [conv, { refetch }] = createResource(activeProject, (p) => (p ? api.conversation(p).catch(() => null) : null));
  let list!: HTMLOListElement;
  let ta!: HTMLTextAreaElement;

  onMount(async () => {
    const off = await on<string>("conversation-changed", (p) => p === activeProject() && refetch());
    // The transcript also grows between hook events while the orchestrator works.
    const t = setInterval(() => conv()?.state === "working" && refetch(), 1500);
    onCleanup(() => {
      off();
      clearInterval(t);
    });
  });

  createEffect(() => {
    const n = conv()?.items.length ?? 0;
    if (n) {
      setPending(null);
      queueMicrotask(() => list && (list.scrollTop = list.scrollHeight));
    }
  });

  const grow = () => {
    ta.style.height = "auto";
    ta.style.height = `${Math.min(ta.scrollHeight, 200)}px`;
  };

  const send = async () => {
    const text = draft().trim();
    if (!text) return;
    if (!info()?.project) return addProject();
    if (!RUNNING.includes(crew.phase)) setCrew({ log: [], review: null, gate: null });
    setPending(text);
    setDraft("");
    grow();
    try {
      await api.say(text);
      setTimeout(refetch, 800);
    } catch (e) {
      setPending(null);
      toast({ title: "Message non envoyé", body: String(e), tone: "danger" });
    }
  };

  const working = () => conv()?.state === "working" || conv()?.state === "starting" || !!pending();

  return (
    <section class="chat">
      <Show when={info()?.project}>
        <header class="chat-bar">
          <span class="label">
            {RUNNING.includes(crew.phase) ? "run en cours" : crew.phase === "done" ? "prêt pour toi" : "au repos"} · {currentProject()?.panes ?? 0} session(s)
          </span>
          <Show when={crew.phase !== "idle"}>
            <RunTrack phase={crew.phase} />
          </Show>
          <span class="spacer" />
          <Show when={conv()?.pane}>
            <button class="icon-btn sm" title="Voir le terminal de l'orchestrateur" onClick={() => setDrawerPane(conv()!.pane)}>
              <Icon icon={D.terminal} size={15} />
            </button>
          </Show>
        </header>
      </Show>

      <ol class="stream" ref={list}>
        <Show when={!(conv()?.items.length) && !pending()}>
          <li class="chat-empty">
            <AsciiField fill />
          </li>
        </Show>
        <For each={conv()?.items ?? []}>{(it) => <Message item={it} />}</For>
        <Show when={pending()}>
          <li class="msg user">
            <div class="msg-meta"><span class="label">toi</span></div>
            <div class="msg-text plain">{pending()}</div>
          </li>
        </Show>
        <Show when={working()}>
          <li class="msg-working">
            <Ascii frame={thinking(14)} />
            <span class="label">l'orchestrateur réfléchit</span>
          </li>
        </Show>
        <Show when={crew.phase === "done" && crew.gate}>
          <li class="ready">
            <span class="label">prêt pour toi · barre du teacher atteinte</span>
            <p>{crew.gate!.summary_for_user}</p>
            <For each={crew.review?.commits ?? []}>
              {(c) => (
                <div class="commit">
                  <code>{c.message}</code>
                  <span class="label">{c.lot} · {c.files.length} fichier(s)</span>
                </div>
              )}
            </For>
            <span class="label faint">rien n'est commité sans ton accord</span>
          </li>
        </Show>
      </ol>

      <div class="chat-bottom">
        <Attention />
        <div class="composer">
          <textarea
            ref={ta}
            id="composer-input"
            rows={1}
            value={draft()}
            placeholder={info()?.project ? "Écris à l'orchestrateur…" : "Ajoute d'abord un projet"}
            onInput={(e) => {
              setDraft(e.currentTarget.value);
              grow();
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                send();
              }
            }}
          />
          <Show when={RUNNING.includes(crew.phase)}>
            <button class="icon-btn" title="Arrêter le run" onClick={() => api.crewCancel()}>
              <Icon icon={D.stop} size={16} />
            </button>
          </Show>
          <button class="send" disabled={!draft().trim()} onClick={send} title="Envoyer">
            <Icon icon={D.send} size={17} />
          </button>
        </div>
      </div>
    </section>
  );
}
