// The thread of a lot: who said what to whom. A reply is announced by three
// typing dots, then the message rises into place under a short arrow that
// draws itself from sender to receiver.

import { For, Show } from "solid-js";
import { Card } from "@/components/ui/card";
import { cx } from "@/lib/cva";
import Icon from "../../../ui/Icon";
import { agents, thread } from "../../data";
import { ic } from "../../icons";
import AgentIcon from "../../ui/AgentIcon";
import { Initials } from "../../ui/parts";

const roleOf = (name: string) => agents.find((a) => a.name === name)?.role ?? (name === "Scout" ? "scout" : undefined);

// Live messages arrive at these ticks; the dots show one tick before.
const ARRIVES = [9, 15];

export default function Threads(props: { t: () => number }) {
  const visible = () => thread.messages.filter((m) => !m.live || props.t() >= ARRIVES[thread.messages.filter((x) => x.live).indexOf(m)]);
  const typing = () => {
    const live = thread.messages.filter((m) => m.live);
    const next = live.findIndex((_, i) => props.t() < ARRIVES[i]);
    return next >= 0 && props.t() >= ARRIVES[next] - 2 ? live[next].from : null;
  };
  return (
    <Card class="min-h-0 gap-0 rounded-lg py-0 shadow-none">
      <header class="flex items-center gap-2.5 border-b px-4 py-2.5">
        <Icon icon={ic.chats} size={16} class="text-muted-foreground" />
        <h2 class="text-sm font-semibold">Thread {thread.lot}</h2>
        <p class="min-w-0 flex-1 truncate text-sm text-muted-foreground">{thread.title}</p>
        <span class="num text-xs text-muted-foreground">{visible().length} messages</span>
      </header>
      <ol class="flex min-h-0 flex-1 flex-col justify-end gap-2.5 overflow-hidden px-4 py-3">
        <For each={visible()}>
          {(m) => (
            <li class={cx("flex gap-2.5", m.live && "m-rise")}>
              <Show when={roleOf(m.from)} fallback={<Initials name="You" class="mt-0.5 size-6" />}>{(r) => <AgentIcon role={r()} size="sm" class="mt-0.5" />}</Show>
              <div class="min-w-0 flex-1">
                <p class="flex items-center gap-1.5 text-xs text-muted-foreground">
                  <span class="font-medium text-foreground">{m.from}</span>
                  <svg viewBox="0 0 14 8" class="h-2 w-3.5" aria-hidden="true"><path d="M0 4h12M9 1l3 3-3 3" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" class={cx(m.live && "m-draw")} style={{ "--len": "20", "--dur": "300ms" }} /></svg>
                  {m.to} · <span class="num">{m.at}</span>
                </p>
                <p class="text-sm leading-snug">{m.text}</p>
              </div>
            </li>
          )}
        </For>
        <Show when={typing()}>
          {(who) => (
            <li class="m-rise flex items-center gap-2.5 text-xs text-muted-foreground">
              <Show when={roleOf(who())}>{(r) => <AgentIcon role={r()} size="sm" class="opacity-70" />}</Show>
              <span class="m-typing" role="status" aria-label={`${who()} is writing`}><i /><i /><i /></span>
            </li>
          )}
        </Show>
      </ol>
    </Card>
  );
}
