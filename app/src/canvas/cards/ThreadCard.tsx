// One thread, read: a fixed header (back, title, who, state), its own scroll area,
// a fixed message box. A long message folds to a digest once a newer one arrives and
// opens in place; the newest is always open. A message that arrives is announced by
// three dots, rises into place, and if you scrolled up, waits behind a "Latest" pill.

import { For, Show, createEffect, createMemo, createSignal, on, onMount } from "solid-js";
import { cx } from "@/lib/cva";
import Icon from "../../ui/Icon";
import ThreadMessage from "../../ui/kit/ThreadMessage";
import { isLong } from "../../lib/prose";
import { ic } from "../icons";
import AgentIcon from "../ui/AgentIcon";
import { createSim } from "../ui/sim";
import { lotThread, type ThreadMsg } from "./threadData";

const gapLabel = (m: number) => (m >= 60 ? `${Math.round(m / 6) / 10} h later` : `${m} min later`);

export default function ThreadCard() {
  const sim = createSim(1000, 22, 6);
  const t = sim.t;
  const [override, setOverride] = createSignal<Record<number, boolean>>({});
  const [unseen, setUnseen] = createSignal(0);
  let scroller!: HTMLDivElement;

  const msgs = createMemo(() => lotThread.messages.filter((m) => m.tick <= t()));
  const count = createMemo(() => msgs().length);
  const lastLong = () => [...msgs()].reverse().find((m) => m.kind === "msg" && isLong(m.text))?.id;
  const isOpen = (m: ThreadMsg) => override()[m.id] ?? m.id === lastLong();
  const typing = () => lotThread.messages.find((m) => m.kind === "msg" && m.tick > 0 && t() < m.tick && t() >= m.tick - 3);
  const closed = () => msgs().some((m) => m.kind === "event");
  const roles = () => [...new Set(lotThread.messages.flatMap((m) => (m.role ? [m.role] : [])))];

  const atBottom = () => scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 80;
  const toLatest = () => {
    const last = scroller.querySelector<HTMLElement>(":scope > ol > li:last-child");
    last?.scrollIntoView({ block: isLong(msgs().at(-1)?.text ?? "") ? "start" : "end", behavior: "smooth" });
    setUnseen(0);
  };
  onMount(() => (scroller.scrollTop = 0));
  createEffect(on(count, (n, prev) => {
    if (prev === undefined) return;
    if (n < prev) return (setOverride({}), setUnseen(0), void (scroller.scrollTop = 0));
    queueMicrotask(() => (atBottom() ? toLatest() : setUnseen((u) => u + 1)));
  }));
  const freshFrom = () => (t() > 0 ? lotThread.messages.filter((m) => m.tick > 0 && m.tick <= t()).map((m) => m.id) : []);

  return (
    <article class="relative flex h-full min-h-0 flex-col overflow-hidden rounded-xl border bg-card">
      <header class="shrink-0 border-b px-4 py-3">
        <div class="flex items-center gap-2.5">
          <button class="grid size-7 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground" aria-label="Back to the threads"><Icon icon={ic.arrowLeft} size={14} /></button>
          <h2 class="min-w-0 flex-1 truncate text-sm font-semibold">{lotThread.title}</h2>
          <span class={cx("inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-xs", closed() ? "text-muted-foreground" : "text-foreground")}>
            <span class={cx("size-1.5 rounded-full", closed() ? "border border-muted-foreground" : "bg-foreground")} />{closed() ? "Closed" : "Open"}
          </span>
        </div>
        <div class="mt-2 flex items-center gap-2 pl-[38px] text-xs text-muted-foreground">
          <span class="flex -space-x-1.5">{<For each={roles()}>{(r) => <AgentIcon role={r} size="sm" class="size-5 rounded ring-2 ring-card" />}</For>}</span>
          <span>Lot {lotThread.lot}</span><span aria-hidden="true">·</span>
          <span class="num">{msgs().length} messages</span><span aria-hidden="true">·</span>
          <span class="num">{msgs().at(-1)!.min} min</span>
        </div>
      </header>

      <div ref={scroller} class="min-h-0 flex-1 overflow-y-auto" onScroll={() => atBottom() && setUnseen(0)}>
        <ol class="flex flex-col gap-4 px-4 py-4">
          <For each={msgs()}>
            {(m, i) => (
              <>
                <Show when={i() > 0 && m.min - msgs()[i() - 1].min >= 10}>
                  <li class="m-rise flex items-center gap-3 text-xs text-muted-foreground" aria-hidden="true"><span class="h-px flex-1 bg-border" />{gapLabel(m.min - msgs()[i() - 1].min)}<span class="h-px flex-1 bg-border" /></li>
                </Show>
                <Show when={m.kind === "msg"} fallback={
                  <li class="m-pop flex items-center justify-center gap-2 py-1 text-xs text-muted-foreground"><Icon icon={ic.check} size={14} />{m.text}<span class="num">· {m.at}</span></li>
                }>
                  <ThreadMessage id={m.id} role={m.role!} from={m.from} to={m.to} at={m.at} text={m.text} open={isOpen(m)} onToggle={() => setOverride((o) => ({ ...o, [m.id]: !isOpen(m) }))} fresh={freshFrom().includes(m.id) && t() - m.tick < 3} />
                </Show>
              </>
            )}
          </For>
          <Show when={typing()}>
            {(w) => (
              <li class="m-rise flex items-center gap-3 text-xs text-muted-foreground">
                <AgentIcon role={w().role!} size="sm" class="opacity-70" />
                <span class="m-typing" role="status" aria-label={`${w().from} is writing`}><i /><i /><i /></span>
              </li>
            )}
          </Show>
        </ol>
      </div>

      <Show when={unseen() > 0}>
        <button class="m-pop absolute bottom-[68px] left-1/2 z-10 inline-flex h-8 -translate-x-1/2 items-center gap-1.5 rounded-full bg-foreground px-3 text-xs font-medium text-background transition-opacity hover:opacity-90" onClick={toLatest}>
          <Icon icon={ic.caret} size={12} />{unseen()} new · Latest
        </button>
      </Show>

      <footer class="flex shrink-0 items-end gap-2 border-t p-2.5">
        <div class="min-h-9 flex-1 rounded-lg border px-3 py-2 text-sm text-muted-foreground">Write in this thread…</div>
        <button class="grid size-9 place-items-center rounded-full bg-muted text-muted-foreground" aria-label="Send" disabled><Icon icon={ic.send} size={16} /></button>
      </footer>
    </article>
  );
}
