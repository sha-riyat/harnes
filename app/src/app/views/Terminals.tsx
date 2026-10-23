// Every terminal of the project in one grid, and the dock that holds one under any
// view. A pane's header carries the agent's icon, its role, "needs you" when it
// waits, the provider switch (context travels in a hand-off file), expand and close.

import { For, Show, createMemo, createResource, createSignal, onCleanup } from "solid-js";
import { api } from "../../api";
import { cx } from "@/lib/cva";
import { dockTabs, drawerPane, focused, maximized, panes, projectPanes, removePane, setDockTabs, setDrawerPane, setFocused, setMaximized, type PaneView } from "../../store";
import TermView from "../../term/TermView";
import Icon from "../../ui/Icon";
import AgentIcon from "../../ui/kit/AgentIcon";
import { nameOf, roleOf, visual } from "../../ui/kit/roles";
import Pop from "../../ui/kit/Pop";
import { ic } from "../../ui/ph";
import { dockHeight, setDockHeight, toast } from "../../ui/state";

const cols = (n: number) => (n <= 1 ? 1 : n <= 4 ? 2 : 3);
const [providers] = createResource(() => api.providers().catch(() => ["claude"]));
const label = (p: PaneView) => (p.key?.startsWith("orch-") ? "Orchestrator" : p.key ? nameOf(p.key, p.title) : p.title.replace(/^\d+-/, ""));

/** Hands the session over to another provider; context travels in a hand-off file. */
function ProviderMenu(props: { pane: PaneView }) {
  const [open, setOpen] = createSignal(false);
  const pick = async (prov: string) => {
    setOpen(false);
    try {
      await api.switchProvider(props.pane.id, prov);
      toast({ title: `Session taken over by ${prov}`, body: "The context was handed over in .harnes/context/.", tone: "ok" });
    } catch (e) {
      toast({ title: "Could not switch", body: String(e), tone: "danger" });
    }
  };
  return (
    <div class="relative">
      <button class="h-6 rounded-md px-2 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground" title="Switch provider (context kept)" onClick={() => setOpen(!open())}>{props.pane.provider ?? "claude"}</button>
      <Pop open={open()} onClose={() => setOpen(false)} align="right" label="Provider" class="w-40 p-1">
        <For each={providers() ?? []}>{(prov) => <button class={cx("flex h-8 w-full items-center rounded-lg px-2.5 text-sm transition-colors hover:bg-accent", prov === (props.pane.provider ?? "claude") && "font-medium")} onClick={() => pick(prov)}>{prov}</button>}</For>
      </Pop>
    </div>
  );
}

function Pane(props: { pane: PaneView }) {
  const p = () => props.pane;
  const isOrch = () => !!p().key?.startsWith("orch-");
  const switchable = () => isOrch() || !!p().key?.startsWith("claude-");
  const state = () => visual(p().state, p().exited);
  const toggleMax = () => setMaximized(maximized() === p().id ? null : p().id);
  return (
    <section class={cx("m-rise flex min-h-0 min-w-0 flex-col overflow-hidden rounded-xl border bg-[var(--term-bg)] transition-colors", focused() === p().id && "border-foreground/40", state() === "blocked" && "border-signal/60")} onMouseDown={() => setFocused(p().id)}>
      <header class="flex h-10 items-center gap-2 border-b bg-card px-3" onDblClick={toggleMax}>
        <Show when={p().key} fallback={<span class="grid size-6 place-items-center rounded-md bg-muted"><Icon icon={ic.terminal} size={13} /></span>}><AgentIcon role={roleOf(p().key, p().role)} state={state()} size="sm" /></Show>
        <span class="min-w-0 truncate text-sm font-medium">{label(p())}</span>
        <Show when={state() === "blocked"}><span class="m-pop inline-flex items-center gap-1.5 text-xs"><span class="size-1.5 rounded-full bg-signal" />needs you</span></Show>
        <span class="flex-1" />
        <Show when={switchable()}><ProviderMenu pane={p()} /></Show>
        <button class="grid size-6 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground" title={maximized() === p().id ? "Shrink" : "Expand"} onClick={toggleMax}><Icon icon={ic.sidebarSimple} size={14} /></button>
        <button class="grid size-6 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground" title="Close" onClick={() => api.close(p().id).finally(() => removePane(p().id))}><Icon icon={ic.xclose} size={14} /></button>
      </header>
      <TermView id={p().id} />
    </section>
  );
}

export default function Terminals() {
  const visible = createMemo(() => {
    const m = maximized();
    const list = projectPanes();
    return m && list.some((p) => p.id === m) ? list.filter((p) => p.id === m) : list;
  });
  return (
    <div class="grid min-h-0 flex-1 gap-3 p-4" style={{ "grid-template-columns": `repeat(${cols(visible().length)}, minmax(0, 1fr))`, "grid-auto-rows": "minmax(0, 1fr)" }}>
      <For each={visible()} fallback={<p class="col-span-full m-auto text-sm text-muted-foreground">No terminal open. Open one from a project in the sidebar.</p>}>{(p) => <Pane pane={p} />}</For>
    </div>
  );
}

/** Terminals docked under the main view: one tab per terminal opened, resizable by the top edge. */
export function TermDock() {
  const tabs = createMemo(() => dockTabs().map((id) => panes.find((p) => p.id === id)).filter((p): p is PaneView => !!p));
  const active = () => tabs().find((p) => p.id === drawerPane());
  const close = (id: string) => {
    const rest = dockTabs().filter((t) => t !== id);
    setDockTabs(rest);
    if (drawerPane() === id) setDrawerPane(rest[rest.length - 1] ?? null);
  };
  let stop: (() => void) | undefined;
  const drag = (e: PointerEvent) => {
    e.preventDefault();
    const y0 = e.clientY, h0 = dockHeight(), max = window.innerHeight - 160;
    const move = (m: PointerEvent) => setDockHeight(Math.max(140, Math.min(max, h0 + y0 - m.clientY)));
    stop = () => (window.removeEventListener("pointermove", move), window.removeEventListener("pointerup", stop!));
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
  };
  onCleanup(() => stop?.());
  return (
    <Show when={active()}>
      {(p) => (
        <section class="relative mx-2 mb-2 flex shrink-0 flex-col overflow-hidden rounded-2xl border bg-[var(--term-bg)]" style={{ height: `${dockHeight()}px` }} aria-label="Terminals">
          <div class="absolute inset-x-0 top-0 z-10 h-1.5 cursor-ns-resize transition-colors hover:bg-foreground/10" onPointerDown={drag} title="Drag to resize" />
          <header class="flex h-9 shrink-0 items-stretch border-b bg-card pr-1.5">
            <For each={tabs()}>
              {(t) => (
                <div class={cx("flex cursor-default items-center gap-2 border-r px-3 text-xs transition-colors", t.id === p().id ? "bg-[var(--term-bg)] text-foreground" : "text-muted-foreground hover:text-foreground")} onClick={() => setDrawerPane(t.id)}>
                  <Show when={t.key} fallback={<Icon icon={ic.terminal} size={13} />}><AgentIcon role={roleOf(t.key, t.role)} state={visual(t.state, t.exited)} size="sm" class="!size-5" /></Show>
                  <span>{label(t)}</span>
                  <button class="grid size-4 place-items-center rounded text-muted-foreground hover:bg-accent hover:text-foreground" title="Close the tab" onClick={(e) => (e.stopPropagation(), close(t.id))}><Icon icon={ic.xclose} size={11} /></button>
                </div>
              )}
            </For>
            <span class="flex-1" />
            <button class="grid w-7 place-items-center text-muted-foreground transition-colors hover:text-foreground" title="Fold" onClick={() => setDrawerPane(null)}><Icon icon={ic.caret} size={14} /></button>
          </header>
          <Show when={p().id} keyed>{(id) => <TermView id={id} />}</Show>
        </section>
      )}
    </Show>
  );
}
