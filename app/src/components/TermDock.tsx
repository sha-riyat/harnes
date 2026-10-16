// Terminals docked under the main view, like an editor's panel: one tab per
// terminal you opened, resizable by its top edge, folded away when not needed.
// Terminals stay hidden until you ask for one.

import { For, Show, createMemo, onCleanup } from "solid-js";
import { dockTabs, drawerPane, panes, setDockTabs, setDrawerPane } from "../store";
import TermView from "../term/TermView";
import { StateMark } from "../ui/ascii/Ascii";
import Icon from "../ui/Icon";
import { icons as D } from "../ui/icons";
import { dockHeight, setDockHeight } from "../ui/state";

const name = (p: { key?: string; title: string }) =>
  p.key?.startsWith("orch-") ? "orchestrator" : p.title.replace(/^\d+-/, "");

export default function TermDock() {
  // Tabs of terminals that still exist.
  const tabs = createMemo(() => dockTabs().map((id) => panes.find((p) => p.id === id)).filter((p) => !!p));
  const active = () => tabs().find((p) => p!.id === drawerPane());

  const close = (id: string) => {
    const rest = dockTabs().filter((t) => t !== id);
    setDockTabs(rest);
    if (drawerPane() === id) setDrawerPane(rest[rest.length - 1] ?? null);
  };

  let stop: (() => void) | undefined;
  const drag = (e: PointerEvent) => {
    e.preventDefault();
    const y0 = e.clientY;
    const h0 = dockHeight();
    const max = window.innerHeight - 160;
    const move = (m: PointerEvent) => setDockHeight(Math.max(140, Math.min(max, h0 + y0 - m.clientY)));
    stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop!);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
  };
  onCleanup(() => stop?.());

  return (
    <Show when={active()}>
      {(p) => (
        <section class="dock" style={{ height: `${dockHeight()}px` }} aria-label="Terminaux">
          <div class="dock-grip" onPointerDown={drag} title="Glisser pour redimensionner" />
          <header class="dock-tabs">
            <For each={tabs()}>
              {(t) => (
                <div class="dock-tab" classList={{ on: t!.id === p()!.id }} onClick={() => setDrawerPane(t!.id)}>
                  <StateMark state={t!.exited ? "exited" : t!.state} id={t!.id} />
                  <span>{name(t!)}</span>
                  <button class="dock-x" title="Fermer l'onglet" onClick={(e) => { e.stopPropagation(); close(t!.id); }}>
                    <Icon icon={D.close} size={12} />
                  </button>
                </div>
              )}
            </For>
            <span class="spacer" />
            <button class="icon-btn sm" title="Replier" onClick={() => setDrawerPane(null)}>
              <Icon icon={D.chevron} size={14} />
            </button>
          </header>
          <Show when={p()!.id} keyed>
            {(id) => <TermView id={id} />}
          </Show>
        </section>
      )}
    </Show>
  );
}
