// Toasts (bottom right) and the "open a project" dialog.

import { For, Show, createSignal } from "solid-js";
import { Button } from "@/components/ui/button";
import { cx } from "@/lib/cva";
import Icon from "../ui/Icon";
import { ic } from "../ui/ph";
import { openPath } from "../ui/projects";
import { dismiss, projectDialog, setProjectDialog, toast, toasts } from "../ui/state";

export function Toasts() {
  return (
    <div class="pointer-events-none fixed right-4 bottom-4 z-40 flex w-[340px] flex-col gap-2" aria-live="polite">
      <For each={toasts}>
        {(t) => (
          <div class="m-rise pointer-events-auto flex gap-3 rounded-xl border bg-popover p-3.5 text-popover-foreground shadow-lg">
            <Icon icon={t.tone === "ok" ? ic.check : t.tone === "danger" ? ic.warn : ic.info} size={18} class={cx("mt-0.5 shrink-0", t.tone === "danger" ? "text-destructive" : t.tone === "ok" ? "text-signal" : "text-muted-foreground")} />
            <div class="min-w-0 flex-1 text-sm leading-snug">
              <p class="font-medium">{t.title}</p>
              <Show when={t.body}><p class="mt-0.5 break-words whitespace-pre-wrap text-muted-foreground">{t.body}</p></Show>
              <Show when={t.action}><Button variant="outline" size="sm" class="mt-2 h-7" onClick={() => (t.action!.run(), dismiss(t.id))}>{t.action!.label}</Button></Show>
            </div>
            <button class="grid size-6 shrink-0 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground" aria-label="Close" onClick={() => dismiss(t.id)}><Icon icon={ic.xclose} size={13} /></button>
          </div>
        )}
      </For>
    </div>
  );
}

export function ProjectDialog() {
  const [path, setPath] = createSignal((() => { try { return localStorage.getItem("harnes.project") ?? ""; } catch { return ""; } })());
  const [busy, setBusy] = createSignal(false);
  const open = async () => {
    if (!path().trim()) return;
    setBusy(true);
    try {
      try { localStorage.setItem("harnes.project", path().trim()); } catch { /* unavailable */ }
      await openPath(path().trim());
      setProjectDialog(false);
    } catch (e) {
      toast({ title: "Could not open it", body: String(e), tone: "danger" });
    } finally {
      setBusy(false);
    }
  };
  return (
    <Show when={projectDialog()}>
      <div class="fixed inset-0 z-50 grid place-items-center bg-black/50" onMouseDown={() => setProjectDialog(false)}>
        <div class="m-menu flex w-[460px] max-w-[92vw] flex-col gap-4 rounded-2xl border bg-popover p-5 text-popover-foreground shadow-2xl" role="dialog" aria-label="Open a project" onMouseDown={(e) => e.stopPropagation()}>
          <div><h2 class="flex items-center gap-2 text-base font-semibold"><Icon icon={ic.folder} size={18} />Open a project</h2><p class="mt-1 text-sm text-muted-foreground">harnes installs the agents, the settings and the MCP servers (harnes-ctx, harnes-e2e) there, then starts the proxy.</p></div>
          <input class="h-10 rounded-lg border bg-transparent px-3 font-mono text-sm outline-none focus-visible:border-foreground/30 focus-visible:ring-[3px] focus-visible:ring-foreground/5" value={path()} placeholder="C:\path\to\project" spellcheck={false} aria-label="Project path" ref={(el) => setTimeout(() => el.focus())} onInput={(e) => setPath(e.currentTarget.value)} onKeyDown={(e) => { if (e.key === "Enter") open(); if (e.key === "Escape") setProjectDialog(false); }} />
          <div class="flex justify-end gap-2"><Button variant="ghost" class="text-muted-foreground" onClick={() => setProjectDialog(false)}>Cancel</Button><Button disabled={busy() || !path().trim()} onClick={open}><Show when={busy()} fallback={<Icon icon={ic.checkBold} size={14} />}><span class="m-breathe size-1.5 rounded-full bg-current" /></Show>Open and equip</Button></div>
        </div>
      </div>
    </Show>
  );
}

export default function Overlays() {
  return (
    <>
      <Toasts />
      <ProjectDialog />
    </>
  );
}
