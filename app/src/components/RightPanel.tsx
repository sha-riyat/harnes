import { For, Show } from "solid-js";
import { crew, stats } from "../store";
import Icon from "../ui/Icon";
import * as I from "../ui/icons";
import { panelTab, setPanelOpen, setPanelTab, type PanelTab } from "../ui/state";
import ThreadsTab from "./ThreadsTab";

const fmt = (n?: number) => (n === undefined ? "–" : n >= 1e6 ? `${(n / 1e6).toFixed(1)}M` : n >= 1e3 ? `${(n / 1e3).toFixed(1)}k` : `${n}`);

const TABS: { id: PanelTab; label: string; icon: typeof I.Team }[] = [
  { id: "crew", label: "Équipage", icon: I.Team },
  { id: "threads", label: "Threads", icon: I.Activity },
  { id: "usage", label: "Coûts", icon: I.Coins },
  { id: "sources", label: "Outils", icon: I.Settings },
];

function CrewTab() {
  return (
    <div class="rp-body">
      <Show when={crew.gate}>
        {(g) => (
          <div class="card ok">
            <h5><Icon icon={I.Check} size={14} /> Porte du Teacher</h5>
            <p class="small">{g().summary_for_user}</p>
          </div>
        )}
      </Show>
      <Show when={crew.review?.commits.length}>
        <div class="card">
          <h5><Icon icon={I.Commit} size={14} /> Commits proposés</h5>
          <p class="muted small">Une action, un commit. À créer après tes tests.</p>
          <For each={crew.review!.commits}>
            {(c) => (
              <div class="commit">
                <code>{c.message}</code>
                <span class="muted small">{c.lot} · {c.files.join(", ")}</span>
              </div>
            )}
          </For>
        </div>
      </Show>
      <h5 class="rp-title"><Icon icon={I.Clock} size={14} /> Journal</h5>
      <ol class="timeline">
        <For each={[...crew.log].reverse()} fallback={<li class="tl-empty muted small">Aucun run pour l'instant. Décris une tâche en bas de l'écran.</li>}>
          {(ev) => (
            <li class={ev.phase}>
              <span class="tl-dot" />
              <span class="tl-phase">{ev.phase}{ev.round ? ` · tour ${ev.round + 1}` : ""}</span>
              <span class="tl-msg">{ev.message}</span>
            </li>
          )}
        </For>
      </ol>
    </div>
  );
}

function UsageTab() {
  const p = () => stats()?.proxy;
  const c = () => stats()?.ctx;
  const tiles = () => [
    { v: p()?.est_tokens_saved, l: "tokens compressés", icon: I.Sparkles },
    { v: p()?.cache_read_tokens, l: "lus en cache", icon: I.Flash },
    { v: p()?.input_tokens, l: "tokens d'entrée", icon: I.ArrowRight },
    { v: p()?.output_tokens, l: "tokens de sortie", icon: I.Send },
    { v: c()?.blobs, l: "sorties isolées", icon: I.Terminal },
    { v: p()?.requests, l: "requêtes", icon: I.Activity },
  ];
  return (
    <div class="rp-body">
      <div class="tiles">
        <For each={tiles()}>
          {(t) => (
            <div class="tile">
              <Icon icon={t.icon} size={14} class="muted" />
              <b>{fmt(t.v)}</b>
              <span>{t.l}</span>
            </div>
          )}
        </For>
      </div>
      <p class="muted small">Compression par le proxy local et isolation des grosses sorties par harnes-ctx.</p>
    </div>
  );
}

function ToolsTab() {
  const tools = [
    { name: "harnes-ctx", desc: "sorties volumineuses hors contexte (MCP)", icon: I.Terminal },
    { name: "harnes-e2e", desc: "tests e2e, objectifs « act » résolus par les agents", icon: I.TestTube },
    { name: "Proxy", desc: "compression déterministe des résultats d'outils", icon: I.Flash },
    { name: "Ponytail · Caveman · TasteSkill", desc: "skills Claude Code", icon: I.Sparkles },
  ];
  return (
    <div class="rp-body">
      <For each={tools}>
        {(t) => (
          <div class="tool-row">
            <Icon icon={t.icon} size={16} class="muted" />
            <div>
              <b>{t.name}</b>
              <p class="muted small">{t.desc}</p>
            </div>
          </div>
        )}
      </For>
    </div>
  );
}

export default function RightPanel() {
  return (
    <aside class="rpanel">
      <div class="rp-tabs" role="tablist">
        <For each={TABS}>
          {(t) => (
            <button role="tab" aria-selected={panelTab() === t.id} classList={{ on: panelTab() === t.id }} onClick={() => setPanelTab(t.id)}>
              <Icon icon={t.icon} size={14} /> {t.label}
            </button>
          )}
        </For>
        <span class="spacer" />
        <button class="icon-btn" title="Fermer" onClick={() => setPanelOpen(false)}>
          <Icon icon={I.Cancel} size={14} />
        </button>
      </div>
      <Show when={panelTab() === "crew"}><CrewTab /></Show>
      <Show when={panelTab() === "threads"}><ThreadsTab /></Show>
      <Show when={panelTab() === "usage"}><UsageTab /></Show>
      <Show when={panelTab() === "sources"}><ToolsTab /></Show>
    </aside>
  );
}
