// The conversation with the project's orchestrator. It runs in a hidden terminal;
// its transcript is the chat. Questions, permissions and the end-of-run summary are
// cards in the stream; the message box carries attach, usage and send.

import { For, Show, createEffect, createSignal } from "solid-js";
import { api, type ConvItem } from "../../api";
import { agents, conv, isRunning, refetchConv } from "../../model";
import { crew, info, setCrew, setDrawerPane } from "../../store";
import Icon from "../../ui/Icon";
import AgentIcon from "../../ui/kit/AgentIcon";
import { Mark } from "../../ui/kit/bits";
import Markdown from "../../ui/Markdown";
import { ic } from "../../ui/ph";
import { addProject } from "../../ui/projects";
import { toast } from "../../ui/state";
import { cx } from "@/lib/cva";
import NeedsYou from "../cards/Decisions";
import { GateCard } from "../cards/Overview";
import Composer from "../Composer";

const time = (iso?: string) => (iso ? new Date(iso).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) : "");

function Message(props: { item: ConvItem }) {
  return (
    <Show
      when={props.item.role !== "tool"}
      fallback={<li class="m-rise flex items-center gap-2 self-start rounded-full border bg-muted/50 px-3 py-1 text-xs text-muted-foreground"><Icon icon={ic.terminal} size={12} /><span class="font-mono">{props.item.tool}</span><span class="max-w-[420px] truncate">{props.item.text}</span></li>}
    >
      <Show
        when={props.item.role === "assistant"}
        fallback={<li class="m-rise max-w-[560px] self-end rounded-2xl bg-muted px-4 py-2.5 text-sm whitespace-pre-wrap">{props.item.text}<span class="num ml-2 text-xs text-muted-foreground">{time(props.item.at)}</span></li>}
      >
        <li class="m-rise flex gap-3">
          <AgentIcon role="orchestrator" state={conv()?.state === "working" ? "working" : undefined} />
          <div class="min-w-0 max-w-[640px] flex-1 text-sm leading-relaxed"><Markdown text={props.item.text} /></div>
        </li>
      </Show>
    </Show>
  );
}

export default function Chat() {
  const [pending, setPending] = createSignal<string | null>(null);
  let list!: HTMLOListElement;

  createEffect(() => {
    const n = conv()?.items.length ?? 0;
    if (n) {
      setPending(null);
      queueMicrotask(() => list && (list.scrollTop = list.scrollHeight));
    }
  });

  const send = async (text: string) => {
    if (!info()?.project) return addProject();
    if (!isRunning()) setCrew({ log: [], review: null, gate: null });
    setPending(text);
    try {
      await api.say(text);
      setTimeout(refetchConv, 800);
    } catch (e) {
      setPending(null);
      toast({ title: "Message not sent", body: String(e), tone: "danger" });
    }
  };
  const working = () => conv()?.state === "working" || conv()?.state === "starting" || !!pending();
  const empty = () => !(conv()?.items.length) && !pending();

  return (
    <section class="flex min-h-0 flex-1 flex-col">
      <ol ref={list} class="mx-auto flex w-full max-w-[760px] min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-6 py-5">
        <Show when={empty()}>
          <li class="m-rise m-auto flex max-w-sm flex-col items-center gap-3 text-center">
            <span class="grid size-12 place-items-center rounded-2xl bg-muted"><Mark class="size-6" /></span>
            <p class="text-base font-semibold">What should the crew build?</p>
            <p class="text-sm text-muted-foreground">{info()?.project ? "Describe it to the orchestrator. It cuts a plan, sets the bar, and asks you only what you must decide." : "Add a project first: harnes sets up its agents and settings there."}</p>
          </li>
        </Show>
        <For each={conv()?.items ?? []}>{(it) => <Message item={it} />}</For>
        <Show when={pending()}><li class="m-rise max-w-[560px] self-end rounded-2xl bg-muted px-4 py-2.5 text-sm whitespace-pre-wrap">{pending()}</li></Show>
        <Show when={working()}>
          <li class="m-rise flex items-center gap-3 text-sm text-muted-foreground"><AgentIcon role="orchestrator" state="working" /><span class="m-typing" role="status" aria-label="The orchestrator is working"><i /><i /><i /></span></li>
        </Show>
        <li class="flex flex-col gap-3 empty:hidden"><NeedsYou /><GateCard /></li>
      </ol>
      <div class="mx-auto w-full max-w-[760px] px-6 pb-5">
        <Composer
          disabled={!info()?.project}
          running={isRunning()}
          onStop={() => api.crewCancel()}
          placeholder={info()?.project ? "Tell the orchestrator what to build…" : "Add a project first"}
          onSend={send}
        />
        <Show when={conv()?.pane}>
          <p class={cx("mt-2 flex items-center gap-1.5 px-2 text-xs text-muted-foreground")}>
            <span>{agents().length} agent{agents().length === 1 ? "" : "s"} in this project ·</span>
            <button class="inline-flex items-center gap-1 transition-colors hover:text-foreground" onClick={() => setDrawerPane(conv()!.pane)}><Icon icon={ic.terminal} size={12} />orchestrator terminal</button>
            <Show when={crew.phase === "done"}><span>· ready for you</span></Show>
          </p>
        </Show>
      </div>
    </section>
  );
}
