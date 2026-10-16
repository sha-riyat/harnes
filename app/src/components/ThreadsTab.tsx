import { For, Show, createResource, createSignal, onCleanup, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { on } from "../api";
import { activeProject } from "../store";
import Icon from "../ui/Icon";
import * as I from "../ui/icons";
import { FlowLine } from "../ui/Ascii";

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

const roleOf = (author: string) => {
  const a = author.toLowerCase();
  for (const r of ["orchestrator", "planner", "teacher", "reviewer", "worker", "scout", "user", "harnes"]) if (a.includes(r)) return r;
  return "other";
};
const short = (author: string) => author.replace(/^\d+-/, "");
const time = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });

/** Live activity: the last messages travelling between agents and threads. */
function Activity(props: { items: Message[] }) {
  return (
    <div class="activity">
      <For each={props.items.slice(-4)} fallback={<span class="muted small">Aucun échange pour l'instant.</span>}>
        {(m) => <FlowLine from={short(m.author)} to={m.thread.replace(/^lot-\d+-/, "").replace(/^run-\d+/, "run")} born={m.at} />}
      </For>
    </div>
  );
}

export default function ThreadsTab() {
  const [filter, setFilter] = createSignal<"open" | "closed" | "all">("all");
  const [current, setCurrent] = createSignal<string | null>(null);
  const [draft, setDraft] = createSignal("");
  const [recent, setRecent] = createSignal<Message[]>([]);

  const [threads, { refetch }] = createResource(() => [filter(), activeProject()] as const, ([f]) => invoke<Thread[]>("threads_list", { status: f }).catch(() => []));
  const [detail, { refetch: refetchDetail }] = createResource(current, (id) =>
    invoke<{ thread: Thread | null; messages: Message[] }>("thread_read", { id }).catch(() => ({ thread: null, messages: [] })),
  );

  let lastId = 0;
  const pullRecent = async () => {
    const list = await invoke<Message[]>("threads_recent", { since: lastId }).catch(() => []);
    if (list.length) {
      lastId = list[list.length - 1].id;
      // Only animate messages younger than a few seconds.
      setRecent((r) => [...r, ...list.map((m) => ({ ...m, at: Math.max(m.at, Date.now() - 2000) }))].slice(-12));
    }
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
      <Activity items={recent()} />
      <Show
        when={current()}
        fallback={
          <>
            <div class="th-filter">
              <For each={[["all", "Tous"], ["open", "Ouverts"], ["closed", "Clos"]] as const}>
                {([v, l]) => (
                  <button classList={{ on: filter() === v }} onClick={() => setFilter(v)}>{l}</button>
                )}
              </For>
            </div>
            <ul class="th-list">
              <For each={threads() ?? []} fallback={<li class="muted small">Aucun fil. Les agents en ouvrent pendant un run.</li>}>
                {(t) => (
                  <li classList={{ closed: t.status === "closed" }} onClick={() => setCurrent(t.id)}>
                    <div class="th-top">
                      <Icon icon={t.status === "closed" ? I.Check : I.Activity} size={13} />
                      <b>{t.title}</b>
                      <span class="muted small">{time(t.last_at)}</span>
                    </div>
                    <div class="th-last muted small">
                      <span class={`who ${roleOf(t.last_author ?? "")}`}>{short(t.last_author ?? "")}</span> {t.summary ?? t.last_body}
                    </div>
                  </li>
                )}
              </For>
            </ul>
          </>
        }
      >
        <div class="th-head">
          <button class="icon-btn" title="Retour" onClick={() => setCurrent(null)}>
            <Icon icon={I.ArrowRight} size={15} style={{ transform: "rotate(180deg)" }} />
          </button>
          <b>{detail()?.thread?.title}</b>
          <span class={`badge ${detail()?.thread?.status === "closed" ? "" : "accent"}`}>{detail()?.thread?.status === "closed" ? "clos" : "ouvert"}</span>
        </div>
        <ol class="th-msgs">
          <For each={detail()?.messages ?? []}>
            {(m) => (
              <li class={`msg ${m.kind}`}>
                <span class={`avatar ${roleOf(m.author)}`}>{short(m.author).slice(0, 1).toUpperCase()}</span>
                <div>
                  <div class="msg-meta">
                    <b class={`who ${roleOf(m.author)}`}>{short(m.author)}</b>
                    <span class="muted small">{time(m.at)}</span>
                  </div>
                  <div class="msg-body">{m.body}</div>
                </div>
              </li>
            )}
          </For>
        </ol>
        <div class="th-input">
          <textarea class="field" rows={2} value={draft()} placeholder="Écrire dans ce fil…" onInput={(e) => setDraft(e.currentTarget.value)} onKeyDown={(e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); send(); } }} />
          <button class="send" disabled={!draft().trim()} onClick={send} title="Envoyer">
            <Icon icon={I.Send} size={16} stroke={2} />
          </button>
        </div>
      </Show>
    </div>
  );
}
