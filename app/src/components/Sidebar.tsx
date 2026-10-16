import { For, Show, createMemo, createSignal } from "solid-js";
import { api, type ProjectSummary } from "../api";
import { activeProject, addPane, crewPhaseOf, info, panesOf, projects, setDrawerPane, setView, type PaneView } from "../store";
import Icon from "../ui/Icon";
import { StateMark } from "../ui/ascii/Ascii";
import { icons as D } from "../ui/icons";
import { addProject, closeProject, switchProject } from "../ui/projects";
import { toast } from "../ui/state";

const STATE: Record<string, string> = { starting: "démarre", working: "travaille", blocked: "t'attend", idle: "prêt", done: "prêt", exited: "arrêté" };
const RUNNING = ["planning", "calibrating", "working", "reviewing", "gating", "fixing"];

function Row(props: { p: PaneView }) {
  const state = () => (props.p.exited ? "exited" : props.p.state ?? "idle");
  const isOrch = () => !!props.p.key?.startsWith("orch-");
  const open = async () => {
    if (props.p.project && props.p.project !== activeProject()) await switchProject(props.p.project);
    if (isOrch()) setView("chat");
    else setDrawerPane(props.p.id);
  };
  return (
    <button class="row" classList={{ attn: state() === "blocked" }} onClick={open} title={isOrch() ? "Conversation" : "Voir le terminal"}>
      <StateMark state={state()} id={props.p.id} />
      <span class="row-name">{isOrch() ? "orchestrateur" : props.p.title.replace(/^\d+-/, "")}</span>
      <span class="label">{props.p.key ? STATE[state()] : "shell"}</span>
    </button>
  );
}

function ProjectNode(props: { p: ProjectSummary }) {
  const active = () => props.p.id === activeProject();
  const [open, setOpen] = createSignal(false);
  const items = createMemo(() => {
    const list = panesOf(props.p.id);
    const rank = (x: PaneView) => (x.key?.startsWith("orch-") ? 0 : x.key && !x.key.startsWith("claude-") ? 1 : 2);
    return [...list].sort((a, b) => rank(a) - rank(b));
  });
  const running = () => RUNNING.includes(crewPhaseOf(props.p.id)) || props.p.running;

  const spawn = async (kind: "shell" | "claude") => {
    try {
      const pane = await api.spawn(kind, props.p.id);
      addPane({ ...pane, project: props.p.id });
      setDrawerPane(pane.id);
    } catch (e) {
      toast({ title: "Ouverture impossible", body: String(e), tone: "danger" });
    }
  };

  return (
    <li class="proj" classList={{ active: active() }}>
      <div class="proj-row" title={props.p.root} onClick={() => (active() ? setOpen(!open()) : switchProject(props.p.id))}>
        <Icon icon={D.folder} size={16} />
        <span class="row-name">{props.p.name}</span>
        <Show when={props.p.waiting > 0}>
          <span class="count" title="Agents qui t'attendent">{props.p.waiting}</span>
        </Show>
        <Show when={running()}>
          <StateMark state="working" id={props.p.id} />
        </Show>
        <span class="proj-actions">
          <button class="icon-btn sm" title="Nouveau terminal" onClick={(e) => { e.stopPropagation(); spawn("shell"); }}>
            <Icon icon={D.terminal} size={14} />
          </button>
          <button class="icon-btn sm" title="Nouvelle session Claude" onClick={(e) => { e.stopPropagation(); spawn("claude"); }}>
            <Icon icon={D.agent} size={14} />
          </button>
          <button class="icon-btn sm" title="Fermer le projet" onClick={(e) => { e.stopPropagation(); closeProject(props.p.id, props.p.name); }}>
            <Icon icon={D.close} size={14} />
          </button>
        </span>
      </div>
      <Show when={active() || open()}>
        <div class="proj-children">
          <For each={items()} fallback={<p class="side-empty">rien en cours</p>}>
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
      <div class="side-head">
        <span class="label">projets</span>
        <span class="spacer" />
        <button class="icon-btn sm" title="Ajouter un projet" onClick={addProject}>
          <Icon icon={D.plus} size={14} />
        </button>
      </div>
      <ul class="proj-tree">
        <For
          each={projects()}
          fallback={
            <button class="row" onClick={addProject}>
              <Icon icon={D.plus} size={14} />
              <span class="row-name">ajouter un projet</span>
            </button>
          }
        >
          {(p) => <ProjectNode p={p} />}
        </For>
      </ul>
      <div class="side-foot">
        <Show when={info() && !info()!.ctx_bin_found}>
          <span class="label">outils absents · npm run ctx</span>
        </Show>
      </div>
    </aside>
  );
}
