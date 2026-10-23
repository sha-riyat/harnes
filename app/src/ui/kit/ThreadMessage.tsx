// One message of a thread: who to whom, when, and the text laid out by Prose. A long
// message shows a digest (its lead and the sections it holds) until it is opened;
// nothing of it is cut, "Read all" shows every word.

import { For, Show, createMemo } from "solid-js";
import { cx } from "@/lib/cva";
import { digest, isLong, parse, words } from "../../lib/prose";
import Icon from "../Icon";
import { ic } from "../ph";
import AgentIcon from "./AgentIcon";
import Prose from "./Prose";
import type { Role } from "./roles";

export interface ThreadMessageProps { id: number; role: Role; from: string; to?: string; at: string; text: string; open: boolean; fresh: boolean; onToggle: () => void }

export default function ThreadMessage(props: ThreadMessageProps) {
  const long = () => isLong(props.text);
  const d = createMemo(() => digest(parse(props.text)));
  return (
    <li class={cx("flex gap-3", props.fresh && "m-rise m-wash -mx-2 rounded-lg px-2 py-1")} data-msg={props.id}>
      <AgentIcon role={props.role} size="sm" class="mt-0.5" />
      <div class="min-w-0 flex-1">
        <p class="flex flex-wrap items-center gap-x-1.5 text-xs text-muted-foreground">
          <span class="text-[13px] font-medium text-foreground">{props.from}</span>
          <Show when={props.to}>
            <svg viewBox="0 0 14 8" class="h-2 w-3.5" aria-hidden="true"><path d="M0 4h12M9 1l3 3-3 3" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" class={cx(props.fresh && "m-draw")} style={{ "--len": "20", "--dur": "300ms" }} /></svg>
            <span>{props.to}</span>
          </Show>
          <span class="num ml-auto">{props.at}</span>
        </p>
        <Show when={long()} fallback={<Prose text={props.text} class="mt-1" />}>
          <div class="m-fold" data-folded={props.open}>
            <div class="mt-1">
              <p class="text-[13.5px] leading-[1.6]">{d().lead}</p>
              <p class="mt-2 flex flex-wrap gap-1.5 text-xs text-muted-foreground">
                <For each={d().labels}>{(l) => <span class="rounded-full border px-2 py-0.5">{l}</span>}</For>
                <Show when={d().items}><span class="rounded-full border px-2 py-0.5">{d().items} steps</span></Show>
              </p>
            </div>
          </div>
          <div class="m-fold" data-folded={!props.open}>
            <div><Prose text={props.text} class="mt-1" /></div>
          </div>
          <button class="-ml-1.5 mt-2 inline-flex h-6 items-center gap-1 rounded-md px-1.5 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground" onClick={props.onToggle} aria-expanded={props.open}>
            <Icon icon={ic.caret} size={12} class={cx("transition-transform duration-200", props.open && "rotate-180")} />
            {props.open ? "Fold" : `Read all · ${words(props.text)} words`}
          </button>
        </Show>
      </div>
    </li>
  );
}
