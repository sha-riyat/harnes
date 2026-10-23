// The message box: one rounded field that runs from the attach button to the send
// button. Attachments sit above the text as chips; the attach menu offers a file or
// a folder; send wakes up once there is something to send.

import { For, Show, createSignal } from "solid-js";
import { Button } from "@/components/ui/button";
import { Kbd } from "@/components/ui/kbd";
import { cx } from "@/lib/cva";
import Icon from "../../ui/Icon";
import { ic } from "../icons";
import ContextPanel from "./ContextPanel";
import { ctx } from "./contextStore";
import Pop from "./Pop";

type Attachment = { name: string; kind: "file" | "folder" };

export default function Composer(props: { class?: string; placeholder?: string; contextOpen?: boolean }) {
  const [text, setText] = createSignal("");
  const [open, setOpen] = createSignal(false);
  const [ctxOpen, setCtxOpen] = createSignal(props.contextOpen ?? false);
  const [items, setItems] = createSignal<Attachment[]>([]);
  const ready = () => text().trim().length > 0 || items().length > 0;
  const add = (a: Attachment) => {
    setItems((l) => (l.some((x) => x.name === a.name) ? l : [...l, a]));
    setOpen(false);
  };
  const send = () => {
    if (!ready()) return;
    setText("");
    setItems([]);
  };
  return (
    <div class={cx("relative", props.class)}>
      <div class="flex flex-col rounded-[22px] border bg-background transition-[border-color,box-shadow] duration-150 focus-within:border-foreground/30 focus-within:ring-[3px] focus-within:ring-foreground/5">
        <Show when={items().length}>
          <ul class="flex flex-wrap gap-1.5 px-3.5 pt-3">
            <For each={items()}>
              {(a) => (
                <li class="m-pop inline-flex h-7 items-center gap-1.5 rounded-full border bg-muted/60 pr-1 pl-2.5 text-xs">
                  <Icon icon={a.kind === "file" ? ic.file : ic.folder} size={13} class="text-muted-foreground" />
                  {a.name}
                  <button class="grid size-5 place-items-center rounded-full text-muted-foreground transition-colors hover:bg-accent hover:text-foreground" aria-label={`Remove ${a.name}`} onClick={() => setItems((l) => l.filter((x) => x !== a))}>
                    <Icon icon={ic.xmark} size={11} />
                  </button>
                </li>
              )}
            </For>
          </ul>
        </Show>
        <div class="flex items-center gap-1 p-1.5">
          <div class="relative">
            <Button variant="ghost" size="icon" class="size-9 rounded-full text-muted-foreground" aria-label="Attach a file or a folder" aria-expanded={open()} onClick={() => (setOpen(!open()), setCtxOpen(false))}>
              <Icon icon={ic.paperclip} size={18} />
            </Button>
            <Pop open={open()} onClose={() => setOpen(false)} above label="Attach" class="w-60 p-1">
              <button class="flex h-9 w-full items-center gap-2.5 rounded-lg px-2.5 text-sm transition-colors hover:bg-accent" onClick={() => add({ name: "brief.pdf", kind: "file" })}>
                <Icon icon={ic.filePlus} size={16} class="text-muted-foreground" />Attach a file<Kbd class="ml-auto">⌘U</Kbd>
              </button>
              <button class="flex h-9 w-full items-center gap-2.5 rounded-lg px-2.5 text-sm transition-colors hover:bg-accent" onClick={() => add({ name: "docs/", kind: "folder" })}>
                <Icon icon={ic.folderPlus} size={16} class="text-muted-foreground" />Attach a folder<Kbd class="ml-auto">⌘⇧U</Kbd>
              </button>
              <p class="px-2.5 pt-2 pb-1 text-xs text-muted-foreground">In this project</p>
              <button class="flex h-8 w-full items-center gap-2.5 rounded-lg px-2.5 text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-foreground" onClick={() => add({ name: "contact.html", kind: "file" })}>
                <Icon icon={ic.file} size={15} />contact.html
              </button>
            </Pop>
          </div>
          <input
            class="h-9 min-w-0 flex-1 bg-transparent px-1.5 text-sm outline-none placeholder:text-muted-foreground"
            placeholder={props.placeholder ?? "Tell the orchestrator what to build…"}
            aria-label="Message"
            value={text()}
            onInput={(e) => setText(e.currentTarget.value)}
            onKeyDown={(e) => e.key === "Enter" && send()}
          />
          <div class="relative">
            <Button variant="ghost" size="icon" class={cx("size-9 rounded-full text-muted-foreground", ctxOpen() && "bg-accent text-foreground")} aria-label={`Context window, ${Math.round(ctx.share() * 100)}% used`} title={`Context ${Math.round(ctx.share() * 100)}% · ${ctx.used().toFixed(1)}k of 1M`} aria-expanded={ctxOpen()} onClick={() => (setCtxOpen(!ctxOpen()), setOpen(false))}>
              <svg viewBox="0 0 24 24" class="size-[22px] -rotate-90" aria-hidden="true">
                <circle cx="12" cy="12" r="8.5" fill="none" class="stroke-border" stroke-width="2.4" />
                <circle cx="12" cy="12" r="8.5" fill="none" class="stroke-foreground" stroke-width="2.4" stroke-linecap="round" stroke-dasharray={`${ctx.share() * 53.4} 53.4`} />
              </svg>
            </Button>
            <Pop open={ctxOpen()} onClose={() => setCtxOpen(false)} above align="right" label="Context window" class="p-0"><ContextPanel /></Pop>
          </div>
          <Button size="icon" aria-label="Send" disabled={!ready()} onClick={send} class={cx("size-9 rounded-full transition-[opacity,background-color] duration-200", !ready() && "bg-muted text-muted-foreground opacity-100")}>
            <Icon icon={ic.send} size={17} />
          </Button>
        </div>
      </div>
    </div>
  );
}
