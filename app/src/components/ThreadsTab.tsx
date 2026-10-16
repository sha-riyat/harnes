import { For, Show, createResource, createSignal, onCleanup, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { on } from "../api";
import { activeProject } from "../store";
import Icon from "../ui/Icon";
import { icons as D } from "../ui/icons";
import Ascii from "../ui/ascii/Ascii";
import { flow } from "../ui/ascii/pieces";

interface Thread {
  id: string;
  title: string;
  kind: string;
  status: string;
  messages: number;
  last_at: number;
  last_author?: string;
  last_body?: string;
  summary?: string;
}
interface Message {
  id: number;
  thread: string;
  author: string;
  at: number;
  kind: string;
  body: string;
}

const who = (a: string) => a.replace(/^\d+-/, "").replace(/^orch-\w+/, "orchestrator").replace(/-r\d+/, "");
const where = (t: string) => t.replace(/^lot-\d+-/, "").replace(/^run-\d+/, "run");
const time = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });

export default function ThreadsTab() {
  const [filter, setFilter] = createSignal<"open" | "closed" | "all">("all");
  const [current, setCurrent] = createSignal<string | null>(null);
  const [draft, setDraft] = createSignal("");
  const [recent, setRecent] = createSignal<(Message & { seen: number })[]>([]);

  const [threads, { refetch }] = createResource(() => [filter(), activeProject()] as const, ([f]) => invoke<Thread[]>("threads_list", { status: f }).catch(() => []));
  const [detail, { refetch: refetchDetail }] = createResource(current, (id) =>
    invoke<{ thread: Thread | null; messages: Message[] }>("thread_read", { id }).catch(() => ({ thread: null, messages: [] })),
  );

  let lastId = 0;
  const pullRecent = async () => {
    const list = await invoke<Message[]>("threads_recent", { since: lastId }).catch(() => []);
    if (!list.length) return;
    lastId = list[list.length - 1].id;
    setRecent((r) => [...r, ...list.map((m) => ({ ...m, seen: Date.now() }))].slice(-3));
  };

  onMount(async () => {
    pullRecent();
    const off = await on<string>("threads-changed", (project) => {
      if (project && project !== activeProject()) return;
      refetch();
      if (current()) refetchDetail();
      pullRecent();
    });
    onCleanup(off);
  });

  const send = async () => {
    if (!current() || !draft().trim()) return;
    await invoke("thread_post", { id: current(), body: draft() }).catch(() => {});
    setDraft("");
    refetchDetail();
  };

  return (
    <div class="rp-body threads">
      <Show when={recent().length}>
        <div class="flows">
          <For each={recent()}>
            {(m) => (
              <span class="flow">
                <span class="flow-from">{who(m.author)}</span>
                <Ascii class="flow-track" frame={flow(14)} fps={20} />
                <span class="flow-to">{where(m.thread)}</span>
              </span>
            )}
          </For>
        </div>
      </Show>

      <Show
        when={current()}
        fallback={
          <>
            <div class="seg">
              <For each={[["all", "all"], ["open", "open"], ["closed", "closed"]] as const}>
                {([v, l]) => (
                  <button classList={{ on: filter() === v }} onClick={() => setFilter(v)}>{l}</button>
                )}
              </For>
            </div>
            <ul class="th-list">
              <For each={threads() ?? []} fallback={<li class="side-empty">Aucun fil. Les agents en ouvrent pendant un run.</li>}>
                {(t) => (
                  <li classList={{ closed: t.status === "closed" }} onClick={() => setCurrent(t.id)}>
                    <div class="th-top">
                      <span class={`th-mark ${t.status}`} />
                      <span class="row-name">{t.title}</span>
                      <span class="label faint">{time(t.last_at)}</span>
                    </div>
                    <div class="th-last">
                      <span class="label">{who(t.last_author ?? "")}</span>
                      <span class="dim">{t.summary ?? t.last_body}</span>
                    </div>
                  </li>
                )}
              </For>
            </ul>
          </>
        }
      >
        <div class="th-head">
          <button class="icon-btn sm" title="Retour" onClick={() => setCurrent(null)}>
            <Icon icon={D.arrowLeft} size={14} />
          </button>
          <span class="row-name">{detail()?.thread?.title}</span>
          <span class="label">{detail()?.thread?.status === "closed" ? "closed" : "open"}</span>
        </div>
        <ol class="th-msgs">
          <For each={detail()?.messages ?? []}>
            {(m) => (
              <li classList={{ event: m.kind === "event" }}>
                <div class="msg-meta">
                  <span class="label">{who(m.author)}</span>
                  <span class="label faint">{time(m.at)}</span>
                </div>
                <div class="th-body">{m.body}</div>
              </li>
            )}
          </For>
        </ol>
        <div class="th-input">
          <textarea class="field" rows={2} value={draft()} placeholder="Écrire dans ce fil…" onInput={(e) => setDraft(e.currentTarget.value)} onKeyDown={(e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); send(); } }} />
          <button class="send" disabled={!draft().trim()} onClick={send} title="Envoyer">
            <Icon icon={D.send} size={16} />
          </button>
        </div>
      </Show>
    </div>
  );
}
