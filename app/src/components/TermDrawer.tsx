// A terminal shown on demand. Terminals stay hidden otherwise.

import { Show } from "solid-js";
import { drawerPane, panes, setDrawerPane } from "../store";
import TermView from "../term/TermView";
import Icon from "../ui/Icon";
import { icons as D } from "../ui/icons";

export default function TermDrawer() {
  const pane = () => panes.find((p) => p.id === drawerPane());
  return (
    <Show when={pane()}>
      {(p) => (
        <div class="overlay drawer-wrap" onMouseDown={() => setDrawerPane(null)}>
          <div class="drawer" onMouseDown={(e) => e.stopPropagation()} role="dialog" aria-label={`Terminal ${p().title}`}>
            <header>
              <Icon icon={D.terminal} size={16} />
              <span class="label">terminal · {p().key?.startsWith("orch-") ? "orchestrateur" : p().title}</span>
              <span class="spacer" />
              <span class="label faint">clic hors du cadre pour fermer</span>
              <button class="icon-btn sm" title="Fermer" onClick={() => setDrawerPane(null)}>
                <Icon icon={D.close} size={15} />
              </button>
            </header>
            <TermView id={p().id} />
          </div>
        </div>
      )}
    </Show>
  );
}
