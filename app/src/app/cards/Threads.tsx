// The threads agents open while they work: a list, and one thread read and answered
// in place. A message that arrives rises into the list with a quiet wash; the author
// is shown by the role's icon. Same data as before (threads_list, thread_read, thread_post).

import { invoke } from "@tauri-apps/api/core";
import { For, Show, createEffect, createMemo, createResource, createSignal, on as track, onCleanup, onMount } from "solid-js";
import { on } from "../../api";
import { Card } from "@/components/ui/card";
import { cx } from "@/lib/cva";
import { activeProject } from "../../store";
import Icon from "../../ui/Icon";
import AgentIcon from "../../ui/kit/AgentIcon";
import ThreadMessage from "../../ui/kit/ThreadMessage";
import { roleOf } from "../../ui/kit/roles";
import { isLong } from "../../lib/prose";
import { ic } from "../../ui/ph";

interface Thread { id: string; title: string; kind: string; status: string; messages: number; last_at: number; last_author?: string; last_body?: string; summary?: string }
interface Message { id: number; thread: string; author: string; at: number; kind: string; body: string }

const who = (a: string) => a.replace(/^\d+-/, "").replace(/^orch-\w+/, "orchestrator").replace(/-r\d+/, "");
const clock = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
const gap = (min: number) => (min >= 60 ? `${Math.round(min / 6) / 10} h later` : `${Math.round(min)} min later`);
const label = (a: string) => { const w = who(a).replace(/-/g, " "); return w[0]?.toUpperCase() + w.slice(1); };

export default function ThreadsCard(props: { class?: string }) {
  const [filter, setFilter] = createSignal<"open" | "closed" | "all">("all");
  const [current, setCurrent] = createSignal<string | null>(null);
  const [draft, setDraft] = createSignal("");
  const [fresh, setFresh] = createSignal<number[]>([]);

  const [threads, { refetch }] = createResource(() => [filter(), activeProject()] as const, ([f]) => invoke<Thread[]>("threads_list", { status: f }).catch(() => []));
  const [detail, { refetch: refetchDetail }] = createResource(current, (id) => invoke<{ thread: Thread | null; messages: Message[] }>("thread_read", { id }).catch(() => ({ thread: null, messages: [] })));

  let lastId = 0;
  const pullRecent = async () => {
    const list = await invoke<Message[]>("threads_recent", { since: lastId }).catch(() => []);
    if (!list.length) return;
    lastId = list[list.length - 1].id;
    setFresh((f) => [...f, ...list.map((m) => m.id)].slice(-20));
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

  // Reading: the newest long message is open, older ones fold to their digest. If the
  // reader is not at the bottom when a message arrives, a pill waits instead of a jump.
  const [override, setOverride] = createSignal<Record<number, boolean>>({});
  const [unseen, setUnseen] = createSignal(0);
  let scroller: HTMLDivElement | undefined;
  const msgs = () => detail()?.messages ?? [];
  const count = createMemo(() => msgs().length);
  const lastLong = () => [...msgs()].reverse().find((m) => m.kind !== "event" && isLong(m.body))?.id;
  const isOpen = (m: Message) => override()[m.id] ?? m.id === lastLong();
  const roles = () => [...new Set(msgs().filter((m) => m.kind !== "event").map((m) => roleOf(m.author)))];
  const span = () => (msgs().length > 1 ? Math.round((msgs()[msgs().length - 1].at - msgs()[0].at) / 60000) : 0);
  const atBottom = () => !scroller || scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 80;
  const toLatest = (smooth = true) => {
    const last = scroller?.querySelector<HTMLElement>(":scope > ol > li:last-child");
    last?.scrollIntoView({ block: isLong(msgs().at(-1)?.body ?? "") ? "start" : "end", behavior: smooth ? "smooth" : "instant" });
    setUnseen(0);
  };
  createEffect(track(current, () => (setOverride({}), setUnseen(0))));
  createEffect(track(count, (n, prev) => {
    if (!n) return;
    const first = prev === undefined || prev === 0;
    queueMicrotask(() => (first || atBottom() ? toLatest(!first) : setUnseen((u) => u + (n - (prev ?? n)))));
  }));

  return (
    <Card class={cx("relative min-h-0 gap-0 overflow-hidden rounded-lg py-0 shadow-none", props.class)}>
      <header class="flex items-center gap-2.5 border-b px-4 py-2.5">
        <Show when={current()} fallback={<Icon icon={ic.chats} size={16} class="text-muted-foreground" />}>
          <button class="grid size-6 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground" aria-label="Back to the threads" onClick={() => setCurrent(null)}><Icon icon={ic.arrowLeft} size={14} /></button>
        </Show>
        <h2 class="min-w-0 flex-1 truncate text-sm font-semibold">{current() ? detail()?.thread?.title ?? "Thread" : "Threads"}</h2>
        <Show when={!current()} fallback={<span class="text-xs text-muted-foreground">{detail()?.thread?.status === "closed" ? "closed" : "open"}</span>}>
          <div class="flex gap-0.5" role="tablist">
            <For each={[["all", "All"], ["open", "Open"], ["closed", "Closed"]] as const}>{([v, l]) => <button role="tab" aria-selected={filter() === v} onClick={() => setFilter(v)} class={cx("h-6 rounded-full px-2.5 text-xs transition-colors", filter() === v ? "bg-accent text-foreground" : "text-muted-foreground hover:text-foreground")}>{l}</button>}</For>
          </div>
        </Show>
      </header>
      <Show
        when={current()}
        fallback={
          <ul class="flex min-h-0 flex-1 flex-col overflow-y-auto">
            <For each={threads() ?? []} fallback={<li class="px-4 py-6 text-sm text-muted-foreground">No thread yet. Agents open them during a run.</li>}>
              {(t) => (
                <li>
                  <button class={cx("flex w-full flex-col gap-0.5 border-b px-4 py-2.5 text-left transition-colors last:border-b-0 hover:bg-accent/50", t.status === "closed" && "opacity-60")} onClick={() => setCurrent(t.id)}>
                    <span class="flex items-center gap-2 text-sm"><span class={cx("size-1.5 rounded-full", t.status === "open" ? "bg-foreground" : "border border-muted-foreground")} /><span class="min-w-0 flex-1 truncate font-medium">{t.title}</span><span class="num text-xs text-muted-foreground">{clock(t.last_at)}</span></span>
                    <span class="truncate text-xs text-muted-foreground"><span class="text-foreground">{label(t.last_author ?? "")}</span> {t.summary ?? t.last_body}</span>
                  </button>
                </li>
              )}
            </For>
          </ul>
        }
      >
        <div class="flex items-center gap-2 border-b px-4 py-2 pl-[52px] text-xs text-muted-foreground">
          <span class="flex -space-x-1.5"><For each={roles()}>{(r) => <AgentIcon role={r} size="sm" class="size-5 rounded ring-2 ring-card" />}</For></span>
          <span class="num">{msgs().length} messages</span><Show when={span()}><span aria-hidden="true">·</span><span class="num">{span()} min</span></Show>
        </div>
        <div ref={scroller} class="relative min-h-0 flex-1 overflow-y-auto" onScroll={() => atBottom() && setUnseen(0)}>
          <ol class="flex flex-col gap-4 px-4 py-4">
            <For each={msgs()}>
              {(m, i) => (
                <>
                  <Show when={i() > 0 && (m.at - msgs()[i() - 1].at) / 60000 >= 10}>
                    <li class="flex items-center gap-3 text-xs text-muted-foreground" aria-hidden="true"><span class="h-px flex-1 bg-border" />{gap((m.at - msgs()[i() - 1].at) / 60000)}<span class="h-px flex-1 bg-border" /></li>
                  </Show>
                  <Show when={m.kind !== "event"} fallback={<li class="flex items-center justify-center gap-2 py-1 text-xs text-muted-foreground"><Icon icon={ic.check} size={14} />{m.body}<span class="num">· {clock(m.at)}</span></li>}>
                    <ThreadMessage id={m.id} role={roleOf(m.author)} from={label(m.author)} at={clock(m.at)} text={m.body} fresh={fresh().includes(m.id)} open={isOpen(m)} onToggle={() => setOverride((o) => ({ ...o, [m.id]: !isOpen(m) }))} />
                  </Show>
                </>
              )}
            </For>
          </ol>
        </div>
        <Show when={unseen() > 0}>
          <button class="m-pop absolute bottom-[68px] left-1/2 z-10 inline-flex h-8 -translate-x-1/2 items-center gap-1.5 rounded-full bg-foreground px-3 text-xs font-medium text-background transition-opacity hover:opacity-90" onClick={() => toLatest()}>
            <Icon icon={ic.caret} size={12} />{unseen()} new · Latest
          </button>
        </Show>
        <div class="flex items-end gap-2 border-t p-2.5">
          <textarea class="min-h-9 flex-1 resize-none rounded-lg border bg-transparent px-3 py-2 text-sm outline-none focus-visible:border-foreground/30" rows={1} placeholder="Write in this thread…" aria-label="Write in this thread" value={draft()} onInput={(e) => setDraft(e.currentTarget.value)} onKeyDown={(e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); send(); } }} />
          <button class="grid size-9 place-items-center rounded-full bg-primary text-primary-foreground transition-opacity disabled:bg-muted disabled:text-muted-foreground" disabled={!draft().trim()} onClick={send} aria-label="Send"><Icon icon={ic.send} size={16} /></button>
        </div>
      </Show>
    </Card>
  );
}
