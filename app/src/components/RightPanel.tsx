import { For, Show } from "solid-js";
import { crew, stats } from "../store";
import Icon from "../ui/Icon";
import { icons as D } from "../ui/icons";
import { panelTab, setPanelOpen, setPanelTab, type PanelTab } from "../ui/state";
import ThreadsTab from "./ThreadsTab";

const fmt = (n?: number) => (n === undefined ? "–" : n >= 1e6 ? `${(n / 1e6).toFixed(1)}M` : n >= 1e3 ? `${(n / 1e3).toFixed(1)}k` : `${n}`);

const TABS: { id: PanelTab; label: string; icon: string }[] = [
  { id: "threads", label: "Threads", icon: D.threads },
  { id: "crew", label: "Log", icon: D.journal },
  { id: "usage", label: "Usage", icon: D.cost },
];

/** What the run did, newest first. Agents themselves live in the sidebar. */
function JournalTab() {
  return (
    <div class="rp-body">
      <ol class="log">
        <For each={[...crew.log].reverse().slice(0, 60)} fallback={<li class="side-empty">Le journal se remplit pendant un run.</li>}>
          {(e) => (
            <li>
              <span class="label">{e.phase}</span>
              <span class="dim">{e.message}</span>
            </li>
          )}
        </For>
      </ol>
    </div>
  );
}

function UsageTab() {
  const p = () => stats()?.proxy;
  const rows = () => [
    ["tokens compressés", p()?.est_tokens_saved],
    ["lus en cache", p()?.cache_read_tokens],
    ["entrée", p()?.input_tokens],
    ["sortie", p()?.output_tokens],
    ["sorties isolées", stats()?.ctx?.blobs],
    ["requêtes", p()?.requests],
  ] as const;
  return (
    <div class="rp-body">
      <div class="figures">
        <For each={rows()}>
          {([label, v]) => (
            <div class="figure">
              <span class="fig-n">{fmt(v)}</span>
              <span class="label">{label}</span>
            </div>
          )}
        </For>
      </div>
    </div>
  );
}

export default function RightPanel() {
  return (
    <aside class="rpanel">
      <div class="rp-tabs" role="tablist">
        <For each={TABS}>
          {(t) => (
            <button role="tab" aria-selected={panelTab() === t.id} aria-label={t.label} title={t.label} classList={{ on: panelTab() === t.id }} onClick={() => setPanelTab(t.id)}>
              <Icon icon={t.icon} size={16} />
            </button>
          )}
        </For>
        <span class="spacer" />
        <button class="icon-btn sm" title="Fermer" onClick={() => setPanelOpen(false)}>
          <Icon icon={D.close} size={14} />
        </button>
      </div>
      <Show when={panelTab() === "threads" || panelTab() === "sources"}><ThreadsTab /></Show>
      <Show when={panelTab() === "crew"}><JournalTab /></Show>
      <Show when={panelTab() === "usage"}><UsageTab /></Show>
    </aside>
  );
}
