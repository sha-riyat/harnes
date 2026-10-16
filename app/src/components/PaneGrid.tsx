import { For, Show, createMemo, createResource, createSignal } from "solid-js";
import { api } from "../api";
import { focused, info, maximized, projectPanes, removePane, setFocused, setMaximized, type PaneView } from "../store";
import TermView from "../term/TermView";
import Icon from "../ui/Icon";
import Ascii, { StateMark } from "../ui/ascii/Ascii";
import { thinking } from "../ui/ascii/pieces";
import { icons as D } from "../ui/icons";
import { toast } from "../ui/state";
import { addProject } from "../ui/projects";
import AsciiField from "../ui/ascii/AsciiField";

function cols(n: number): number {
  if (n <= 1) return 1;
  if (n <= 4) return 2;
  return 3;
}

const [providers] = createResource(() => api.providers().catch(() => ["claude"]));

/** Hands the session over to another provider; context travels in a hand-off file. */
function ProviderMenu(props: { pane: PaneView }) {
  const [open, setOpen] = createSignal(false);
  const pick = async (prov: string) => {
    setOpen(false);
    try {
      await api.switchProvider(props.pane.id, prov);
      toast({ title: `Session reprise par ${prov}`, body: "Le contexte a été transmis dans .harnes/context/.", tone: "ok" });
    } catch (e) {
      toast({ title: "Changement impossible", body: String(e), tone: "danger" });
    }
  };
  return (
    <div class="prov-menu">
      <button class="prov-btn" title="Changer de fournisseur (contexte conservé)" onClick={() => setOpen(!open())}>
        {props.pane.provider ?? "claude"}
      </button>
      <Show when={open()}>
        <ul class="menu" onMouseLeave={() => setOpen(false)}>
          <For each={providers() ?? []}>
            {(prov) => (
              <li classList={{ on: prov === (props.pane.provider ?? "claude") }} onClick={() => pick(prov)}>
                {prov}
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}

function Pane(props: { pane: PaneView }) {
  const p = () => props.pane;
  const isOrch = () => !!p().key?.startsWith("orch-");
  const switchable = () => isOrch() || !!p().key?.startsWith("claude-");
  const state = () => (p().exited ? "exited" : p().state ?? "idle");
  const toggleMax = () => setMaximized(maximized() === p().id ? null : p().id);
  return (
    <section class="pane" classList={{ focused: focused() === p().id, exited: !!p().exited, waiting: state() === "blocked" }} onMouseDown={() => setFocused(p().id)}>
      <header onDblClick={toggleMax}>
        <Show when={state() === "working" || state() === "starting"} fallback={<StateMark state={state()} id={p().id} />}>
          <Ascii frame={thinking(4)} />
        </Show>
        <Icon icon={isOrch() ? D.orchestrator : p().key ? D.agent : D.terminal} size={14} />
        <span class="title">{isOrch() ? "orchestrateur" : p().title}</span>
        <Show when={p().role && !isOrch() && p().role !== "claude"}>
          <span class={`role ${p().role}`}>{p().role}</span>
        </Show>
        <Show when={state() === "blocked"}>
          <span class="badge warn">t'attend</span>
        </Show>
        <span class="spacer" />
        <Show when={switchable()}>
          <ProviderMenu pane={p()} />
        </Show>
        <button class="icon-btn sm" title={maximized() === p().id ? "Réduire" : "Agrandir"} onClick={toggleMax}>
          <Icon icon={D.expand} size={14} />
        </button>
        <button class="icon-btn sm" title="Fermer" onClick={() => api.close(p().id).finally(() => removePane(p().id))}>
          <Icon icon={D.close} size={14} />
        </button>
      </header>
      <TermView id={p().id} />
    </section>
  );
}

function Empty() {
  return (
    <div class="empty">
      <AsciiField height={120} />
      <Show
        when={info()?.project}
        fallback={
          <>
            <h2>Ouvre un projet pour commencer</h2>
            <p class="muted">harnes y installe l'équipage, ses outils et ses tests.</p>
            <button class="btn primary" onClick={addProject}>
              <Icon icon={D.folder} size={15} /> Ouvrir un projet
            </button>
          </>
        }
      >
        <h2>Que doit construire l'équipage ?</h2>
        <p class="muted">Parle à l'orchestrateur en bas. Il lance l'équipage ; chaque agent apparaît ici dans son terminal.</p>
      </Show>
    </div>
  );
}

export default function PaneGrid() {
  const visible = createMemo(() => {
    const m = maximized();
    const list = projectPanes();
    return m && list.some((p) => p.id === m) ? list.filter((p) => p.id === m) : list;
  });
  return (
    <div class="grid" style={{ "grid-template-columns": `repeat(${cols(visible().length)}, minmax(0, 1fr))` }}>
      <For each={visible()} fallback={<Empty />}>
        {(p) => <Pane pane={p} />}
      </For>
    </div>
  );
}
