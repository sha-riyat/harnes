import { For, Show, createMemo, createSignal } from "solid-js";
import { api, type ProjectSummary } from "../api";
import { activeProject, addPane, crewPhaseOf, focused, info, panesOf, projects, setFocused, setView, type PaneView } from "../store";
import Icon from "../ui/Icon";
import * as I from "../ui/icons";
import { addProject, closeProject, switchProject } from "../ui/projects";
import { toast } from "../ui/state";
import { Spinner } from "../ui/Ascii";

const STATE_LABEL: Record<string, string> = {
  starting: "démarre",
  working: "au travail",
  blocked: "t'attend",
  idle: "prêt",
  done: "tour fini",
  exited: "arrêté",
};
const RUNNING = ["planning", "calibrating", "working", "reviewing", "gating", "fixing"];

function Row(props: { p: PaneView }) {
  const state = () => (props.p.exited ? "exited" : props.p.state ?? "idle");
  const isOrch = () => props.p.key?.startsWith("orch-");
  return (
    <button
      class="side-row child"
      classList={{ on: focused() === props.p.id, warn: state() === "blocked" }}
      onClick={() => {
        if (props.p.project && props.p.project !== activeProject()) switchProject(props.p.project);
        setFocused(props.p.id);
        setView("panes");
      }}
    >
      <Show when={state() === "working" || state() === "starting"} fallback={<span class={`dot ${state()}`} />}>
        <Spinner class="accent" />
      </Show>
      <Icon icon={isOrch() ? I.Brain : props.p.key ? I.Robot : I.Terminal} size={13} class="muted" />
      <span class="name">{isOrch() ? "orchestrateur" : props.p.title}</span>
      <Show when={props.p.role && !isOrch() && props.p.role !== "claude"}>
        <span class={`role ${props.p.role}`}>{props.p.role}</span>
      </Show>
      <span class="state muted">{props.p.key ? STATE_LABEL[state()] : ""}</span>
    </button>
  );
}

function ProjectNode(props: { p: ProjectSummary }) {
  const isActive = () => props.p.id === activeProject();
  const [open, setOpen] = createSignal(false);
  const expanded = () => isActive() || open();
  const items = createMemo(() => {
    const list = panesOf(props.p.id);
    const orch = list.filter((x) => x.key?.startsWith("orch-"));
    const crew = list.filter((x) => x.key && !x.key.startsWith("orch-") && !x.key.startsWith("claude-"));
    const sessions = list.filter((x) => !x.key || x.key.startsWith("claude-"));
    return [...orch, ...crew, ...sessions];
  });
  const running = () => RUNNING.includes(crewPhaseOf(props.p.id)) || props.p.running;

  const spawn = async (kind: "shell" | "claude") => {
    try {
      const pane = await api.spawn(kind, props.p.id);
      addPane({ ...pane, project: props.p.id });
      if (!isActive()) await switchProject(props.p.id);
      setFocused(pane.id);
      setView("panes");
    } catch (e) {
      toast({ title: "Impossible d'ouvrir le terminal", body: String(e), tone: "danger" });
    }
  };

  return (
    <li class="project" classList={{ active: isActive() }}>
      <div class="project-row" onClick={() => (isActive() ? setOpen(!open()) : switchProject(props.p.id))} title={props.p.root}>
        <Icon icon={I.ArrowRight} size={12} class="chev" style={{ transform: expanded() ? "rotate(90deg)" : "none" }} />
        <Icon icon={I.Folder} size={15} />
        <span class="name">{props.p.name}</span>
        <Show when={running()}>
          <Spinner class="accent" />
        </Show>
        <Show when={props.p.waiting > 0}>
          <span class="badge warn" title="Agents qui t'attendent">{props.p.waiting}</span>
        </Show>
        <span class="row-actions">
          <button class="icon-btn sm" title="Nouveau terminal" onClick={(e) => { e.stopPropagation(); spawn("shell"); }}>
            <Icon icon={I.Terminal} size={13} />
          </button>
          <button class="icon-btn sm" title="Nouvelle session Claude" onClick={(e) => { e.stopPropagation(); spawn("claude"); }}>
            <Icon icon={I.Robot} size={13} />
          </button>
          <button class="icon-btn sm" title="Fermer le projet" onClick={(e) => { e.stopPropagation(); closeProject(props.p.id, props.p.name); }}>
            <Icon icon={I.Cancel} size={13} />
          </button>
        </span>
      </div>
      <Show when={expanded()}>
        <div class="project-children">
          <For each={items()} fallback={<p class="side-empty">Rien en cours. Parle à l'orchestrateur en bas.</p>}>
            {(x) => <Row p={x} />}
          </For>
        </div>
      </Show>
    </li>
  );
}

export default function Sidebar() {
  return (
    <aside class="sidebar">
      <button class="new-task" onClick={() => document.getElementById("composer-input")?.focus()} disabled={!info()?.project}>
        <Icon icon={I.Sparkles} size={16} />
        Nouvelle tâche
      </button>

      <section class="side-sec">
        <h4>
          <Icon icon={I.Folder} size={13} /> Projets
          <span class="spacer" />
          <button class="icon-btn sm" title="Ajouter un projet" onClick={addProject}>
            <Icon icon={I.Add} size={14} />
          </button>
        </h4>
        <ul class="project-tree">
          <For
            each={projects()}
            fallback={
              <button class="side-action" onClick={addProject}>
                <Icon icon={I.Add} size={14} /> Ajouter un projet
              </button>
            }
          >
            {(p) => <ProjectNode p={p} />}
          </For>
        </ul>
      </section>

      <div class="side-foot">
        <Show when={info()?.project}>
          <div class="foot-line muted small">
            <Icon icon={I.Flash} size={12} /> proxy :{info()?.proxy_port} · hooks :{info()?.hook_port}
          </div>
          <Show when={!info()?.ctx_bin_found}>
            <div class="foot-line small" style={{ color: "var(--warn)" }}>
              <Icon icon={I.Alert} size={12} /> outils absents : npm run ctx
            </div>
          </Show>
        </Show>
      </div>
    </aside>
  );
}
