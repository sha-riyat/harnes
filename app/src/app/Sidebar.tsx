// The sidebar: projects (the active one opens onto its agents), what the week has
// left, and the two areas outside a project, Methods and Settings. A count appears
// only on a queue that needs the user.

import { For, Show, createMemo, createSignal } from "solid-js";
import { api, type ProjectSummary } from "../api";
import { Progress, ProgressGroup, ProgressLabel, ProgressValueLabel } from "@/components/ui/progress";
import { cx } from "@/lib/cva";
import { RUNNING, budget, left } from "../model";
import { activeProject, addPane, crewPhaseOf, info, panesOf, projects, setDrawerPane, setView, stats, view, type PaneView } from "../store";
import Icon from "../ui/Icon";
import AgentIcon from "../ui/kit/AgentIcon";
import { nameOf, roleOf, visual, visualLabel } from "../ui/kit/roles";
import { ic } from "../ui/ph";
import { addProject, closeProject, switchProject } from "../ui/projects";
import { toast } from "../ui/state";

const fmt = (n?: number) => (n === undefined ? "–" : n >= 1e6 ? `${(n / 1e6).toFixed(1)}M` : n >= 1e3 ? `${(n / 1e3).toFixed(1)}k` : `${n}`);

function Row(props: { p: PaneView }) {
  const state = () => visual(props.p.state, props.p.exited);
  const isOrch = () => !!props.p.key?.startsWith("orch-");
  const open = async () => {
    if (props.p.project && props.p.project !== activeProject()) await switchProject(props.p.project);
    if (isOrch()) setView("chat");
    else setDrawerPane(props.p.id);
  };
  return (
    <button class="flex h-9 w-full items-center gap-2.5 rounded-lg px-2 text-sm text-muted-foreground transition-colors hover:bg-accent/60 hover:text-foreground" onClick={open} title={isOrch() ? "Conversation" : "Open the terminal"}>
      <Show when={props.p.key} fallback={<span class="grid size-6 place-items-center rounded-md bg-muted"><Icon icon={ic.terminal} size={13} /></span>}>
        <AgentIcon role={roleOf(props.p.key, props.p.role)} state={state()} size="sm" />
      </Show>
      <span class="min-w-0 flex-1 truncate text-left">{isOrch() ? "Orchestrator" : props.p.key ? nameOf(props.p.key, props.p.title) : props.p.title.replace(/^\d+-/, "")}</span>
      <Show when={state() === "blocked"}><span class="m-pop text-xs text-foreground">{visualLabel.blocked}</span></Show>
    </button>
  );
}

function ProjectNode(props: { p: ProjectSummary }) {
  const active = () => props.p.id === activeProject();
  const [open, setOpen] = createSignal(false);
  const items = createMemo(() => {
    const rank = (x: PaneView) => (x.key?.startsWith("orch-") ? 0 : x.key && !x.key.startsWith("claude-") ? 1 : 2);
    return [...panesOf(props.p.id)].sort((a, b) => rank(a) - rank(b));
  });
  const running = () => RUNNING.includes(crewPhaseOf(props.p.id)) || props.p.running;
  const spawn = async (kind: "shell" | "claude") => {
    try {
      const pane = await api.spawn(kind, props.p.id);
      addPane({ ...pane, project: props.p.id });
      setDrawerPane(pane.id);
    } catch (e) {
      toast({ title: "Could not open it", body: String(e), tone: "danger" });
    }
  };
  const act = (title: string, icon: string, fn: () => void) => (
    <button class="grid size-6 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-background hover:text-foreground" title={title} aria-label={title} onClick={(e) => (e.stopPropagation(), fn())}><Icon icon={icon} size={13} /></button>
  );
  return (
    <li>
      <div class={cx("group flex h-8 cursor-default items-center gap-2.5 rounded-lg px-2.5 text-sm transition-colors", active() ? "bg-accent text-foreground" : "text-muted-foreground hover:bg-accent/60 hover:text-foreground")} title={props.p.root} onClick={() => (!active() ? switchProject(props.p.id) : ["os", "settings"].includes(view()) ? setView("overview") : setOpen(!open()))}>
        <Icon icon={ic.folder} size={16} />
        <span class="min-w-0 flex-1 truncate">{props.p.name}</span>
        <Show when={running()}><span class="m-breathe size-1.5 rounded-full bg-foreground" title="A run is going" /></Show>
        <span class="hidden items-center gap-0.5 group-hover:flex">
          {act("New terminal", ic.terminal, () => spawn("shell"))}
          {act("New Claude session", ic.robot, () => spawn("claude"))}
          {act("Close the project", ic.xclose, () => closeProject(props.p.id, props.p.name))}
        </span>
        <Show when={props.p.waiting > 0}><span class="num grid h-5 min-w-5 place-items-center rounded-full bg-foreground px-1.5 text-[11px] font-medium text-background" title="Agents waiting for you">{props.p.waiting}</span></Show>
      </div>
      <Show when={active() || open()}>
        <div class="m-rise ml-3.5 flex flex-col gap-0.5 border-l py-1 pl-2">
          <For each={items()} fallback={<p class="px-2.5 py-1 text-xs text-muted-foreground">No agent yet</p>}>{(x) => <Row p={x} />}</For>
        </div>
      </Show>
    </li>
  );
}

function Link(props: { icon: string; label: string; on: boolean; onClick: () => void }) {
  return <button class={cx("flex h-8 w-full items-center gap-2.5 rounded-lg px-2.5 text-sm transition-colors", props.on ? "bg-accent text-foreground" : "text-muted-foreground hover:bg-accent/60 hover:text-foreground")} onClick={props.onClick}><Icon icon={props.icon} size={16} />{props.label}</button>;
}

export default function Sidebar() {
  return (
    <aside class="flex w-[228px] shrink-0 flex-col gap-4 overflow-y-auto px-3 pt-1 pb-3" aria-label="Main">
      <div class="flex flex-col gap-0.5">
        <div class="flex items-center px-2.5 pb-1"><p class="flex-1 text-xs text-muted-foreground">Projects</p><button class="grid size-5 place-items-center rounded text-muted-foreground transition-colors hover:bg-accent hover:text-foreground" title="Add a project" aria-label="Add a project" onClick={addProject}><Icon icon={ic.plus} size={13} /></button></div>
        <ul class="flex flex-col gap-0.5">
          <For each={projects()} fallback={<button class="flex h-8 items-center gap-2.5 rounded-lg px-2.5 text-sm text-muted-foreground transition-colors hover:bg-accent/60 hover:text-foreground" onClick={addProject}><Icon icon={ic.plus} size={15} />Add a project</button>}>{(p) => <ProjectNode p={p} />}</For>
        </ul>
      </div>
      <div class="mt-auto flex flex-col gap-3">
        <Show when={budget()?.limits}>
          {(l) => (
            <div class="flex flex-col gap-2 px-2.5">
              <Progress value={l().seven_day * 100} class="gap-1.5"><ProgressGroup><ProgressLabel class="text-xs font-normal text-muted-foreground">Week · {left(l().seven_day_reset)}</ProgressLabel><ProgressValueLabel class="num text-xs" /></ProgressGroup></Progress>
              <Show when={stats()?.proxy}><p class="num text-xs text-muted-foreground">{fmt(stats()!.proxy!.est_tokens_saved)} tokens saved</p></Show>
            </div>
          )}
        </Show>
        <Show when={info() && !info()!.ctx_bin_found}><p class="px-2.5 text-xs text-muted-foreground">Tools missing · run npm run ctx</p></Show>
        <div class="flex flex-col gap-0.5">
          <Link icon={ic.flask} label="Methods" on={view() === "os"} onClick={() => setView("os")} />
          <Link icon={ic.gear} label="Settings" on={view() === "settings"} onClick={() => setView("settings")} />
        </div>
      </div>
    </aside>
  );
}
