// The message box: one rounded field from the attach button to the send button.
// Attach opens the system picker for a file or a folder; what is picked is added to
// the message as an @path mention, which the agent reads. The ring is the account's
// 5-hour usage; it opens the usage panel. While a run is going the send button is
// joined by a stop.

import { open } from "@tauri-apps/plugin-dialog";
import { For, Show, createSignal } from "solid-js";
import { Button } from "@/components/ui/button";
import { Kbd } from "@/components/ui/kbd";
import { cx } from "@/lib/cva";
import { budget } from "../model";
import Icon from "../ui/Icon";
import Pop from "../ui/kit/Pop";
import { ic } from "../ui/ph";
import { toast } from "../ui/state";
import { UsagePanel } from "./Panels";

type Attachment = { path: string; kind: "file" | "folder" };
const baseName = (p: string) => p.replace(/[\\/]+$/, "").split(/[\\/]/).pop() ?? p;

export default function Composer(props: {
  class?: string;
  placeholder?: string;
  disabled?: boolean;
  running?: boolean;
  onSend: (text: string) => void | Promise<void>;
  onStop?: () => void;
}) {
  const [text, setText] = createSignal("");
  const [menu, setMenu] = createSignal(false);
  const [usage, setUsage] = createSignal(false);
  const [items, setItems] = createSignal<Attachment[]>([]);
  let area!: HTMLTextAreaElement;
  const ready = () => !props.disabled && (text().trim().length > 0 || items().length > 0);
  const share = () => budget()?.limits.five_hour ?? 0;

  const grow = () => {
    area.style.height = "auto";
    area.style.height = `${Math.min(area.scrollHeight, 200)}px`;
  };
  const pick = async (kind: "file" | "folder") => {
    setMenu(false);
    try {
      const picked = await open({ directory: kind === "folder", multiple: kind === "file", title: kind === "folder" ? "Attach a folder" : "Attach a file" });
      const list = Array.isArray(picked) ? picked : picked ? [picked] : [];
      setItems((l) => [...l, ...list.filter((p) => !l.some((x) => x.path === p)).map((path) => ({ path, kind }))]);
    } catch (e) {
      toast({ title: "Could not open the picker", body: String(e), tone: "danger" });
    }
  };
  const send = async () => {
    if (!ready()) return;
    const mentions = items().map((a) => `@${a.path}`).join(" ");
    const body = [text().trim(), mentions].filter(Boolean).join("\n");
    setText("");
    setItems([]);
    queueMicrotask(grow);
    await props.onSend(body);
  };

  return (
    <div class={cx("relative", props.class)}>
      <div class="flex flex-col rounded-[22px] border bg-background transition-[border-color,box-shadow] duration-150 focus-within:border-foreground/30 focus-within:ring-[3px] focus-within:ring-foreground/5">
        <Show when={items().length}>
          <ul class="flex flex-wrap gap-1.5 px-3.5 pt-3">
            <For each={items()}>
              {(a) => (
                <li class="m-pop inline-flex h-7 items-center gap-1.5 rounded-full border bg-muted/60 pr-1 pl-2.5 text-xs" title={a.path}>
                  <Icon icon={a.kind === "file" ? ic.file : ic.folder} size={13} class="text-muted-foreground" />
                  {baseName(a.path)}
                  <button class="grid size-5 place-items-center rounded-full text-muted-foreground transition-colors hover:bg-accent hover:text-foreground" aria-label={`Remove ${baseName(a.path)}`} onClick={() => setItems((l) => l.filter((x) => x !== a))}>
                    <Icon icon={ic.xmark} size={11} />
                  </button>
                </li>
              )}
            </For>
          </ul>
        </Show>
        <div class="flex items-end gap-1 p-1.5">
          <div class="relative">
            <Button variant="ghost" size="icon" class="size-9 rounded-full text-muted-foreground" aria-label="Attach a file or a folder" aria-expanded={menu()} onClick={() => (setMenu(!menu()), setUsage(false))}>
              <Icon icon={ic.paperclip} size={18} />
            </Button>
            <Pop open={menu()} onClose={() => setMenu(false)} above label="Attach" class="w-56 p-1">
              <button class="flex h-9 w-full items-center gap-2.5 rounded-lg px-2.5 text-sm transition-colors hover:bg-accent" onClick={() => pick("file")}>
                <Icon icon={ic.filePlus} size={16} class="text-muted-foreground" />Attach a file<Kbd class="ml-auto">⌘U</Kbd>
              </button>
              <button class="flex h-9 w-full items-center gap-2.5 rounded-lg px-2.5 text-sm transition-colors hover:bg-accent" onClick={() => pick("folder")}>
                <Icon icon={ic.folderPlus} size={16} class="text-muted-foreground" />Attach a folder<Kbd class="ml-auto">⌘⇧U</Kbd>
              </button>
            </Pop>
          </div>
          <textarea
            ref={area}
            id="composer-input"
            rows={1}
            class="max-h-[200px] min-h-9 min-w-0 flex-1 resize-none bg-transparent px-1.5 py-2 text-sm leading-5 outline-none placeholder:text-muted-foreground disabled:opacity-50"
            placeholder={props.placeholder ?? "Tell the orchestrator what to build…"}
            aria-label="Message"
            disabled={props.disabled}
            value={text()}
            onInput={(e) => (setText(e.currentTarget.value), grow())}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                send();
              }
            }}
          />
          <div class="relative">
            <Button variant="ghost" size="icon" class={cx("size-9 rounded-full text-muted-foreground", usage() && "bg-accent text-foreground")} aria-label={`Usage, ${Math.round(share() * 100)}% of the 5-hour window`} title={budget() ? `5-hour window ${Math.round(share() * 100)}% used` : "Usage appears after the first request"} aria-expanded={usage()} onClick={() => (setUsage(!usage()), setMenu(false))}>
              <svg viewBox="0 0 24 24" class="size-[22px] -rotate-90" aria-hidden="true">
                <circle cx="12" cy="12" r="8.5" fill="none" class="stroke-border" stroke-width="2.4" />
                <circle cx="12" cy="12" r="8.5" fill="none" class="stroke-foreground transition-[stroke-dasharray] duration-500" stroke-width="2.4" stroke-linecap="round" stroke-dasharray={`${share() * 53.4} 53.4`} />
              </svg>
            </Button>
            <Pop open={usage()} onClose={() => setUsage(false)} above align="right" label="Usage" class="p-0"><UsagePanel /></Pop>
          </div>
          <Show when={props.running}>
            <Button variant="outline" size="icon" class="size-9 rounded-full" aria-label="Stop the run" title="Stop the run" onClick={props.onStop}><Icon icon={ic.stop} size={16} /></Button>
          </Show>
          <Button size="icon" aria-label="Send" disabled={!ready()} onClick={send} class={cx("size-9 rounded-full transition-[opacity,background-color] duration-200", !ready() && "bg-muted text-muted-foreground opacity-100")}>
            <Icon icon={ic.send} size={17} />
          </Button>
        </div>
      </div>
    </div>
  );
}
