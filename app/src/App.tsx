import { Match, Show, Switch, onCleanup, onMount } from "solid-js";
import { api, on, subscribeFrames, type AgentState, type CrewEvent, type PaneInfo, type PermissionAsk, type ScreenPrompt } from "./api";
import Conversation from "./components/Conversation";
import LookoutView from "./components/LookoutView";
import MethodsView from "./components/MethodsView";
import { ProjectDialog, Toasts } from "./components/Overlays";
import Palette from "./components/Palette";
import PaneGrid from "./components/PaneGrid";
import RightPanel from "./components/RightPanel";
import RunView from "./components/RunView";
import Sidebar from "./components/Sidebar";
import TermDock from "./components/TermDock";
import TopBar from "./components/TopBar";
import { activeProject, addPane, markExited, pushCrew, pushFrame, removePane, setAgentState, setPermissions, setScreenPrompts, setStats, view } from "./store";
import { refreshProjects } from "./ui/projects";
import { applyTheme, panelOpen, sidebarOpen, toast } from "./ui/state";

function onCrew(ev: CrewEvent & { project?: string }) {
  pushCrew(ev);
  const elsewhere = ev.project && ev.project !== activeProject() ? " · autre projet" : "";
  if (ev.phase === "done") toast({ title: `Prêt pour toi${elsewhere}`, body: ev.gate?.summary_for_user, tone: "ok" }, 0);
  if (ev.phase === "failed") toast({ title: `Run arrêté${elsewhere}`, body: ev.message, tone: "danger" }, 0);
}

function onScreenPrompt(p: ScreenPrompt) {
  setScreenPrompts((l) => [...l.filter((x) => x.pane !== p.pane), ...(p.prompt ? [p] : [])]);
}

export default function App() {
  applyTheme();
  onMount(async () => {
    const offs = await Promise.all([
      on<{ info: PaneInfo; key?: string; role?: string; provider?: string; project?: string }>("pane-open", (p) =>
        addPane({ ...p.info, project: p.project ?? p.info.project, key: p.key, role: p.role, provider: p.provider, state: p.key ? "starting" : undefined }),
      ),
      on("projects-changed", () => refreshProjects()),
      on<string | null>("project-active", () => refreshProjects()),
      on<{ projects: number; panes: number; runs: number; errors: string[] }>("restore", (r) => {
        refreshProjects();
        if (r.errors?.length) toast({ title: "Restauration partielle", body: r.errors.join("\n"), tone: "warn" });
        if (r.projects) toast({ title: "Session restaurée", body: `${r.projects} projet(s) · ${r.panes} session(s)${r.runs ? ` · ${r.runs} run(s) repris` : ""}`, tone: "ok" });
      }),
      on<string>("pane-closed", (id) => removePane(id)),
      on<{ id: string }>("pane-exit", (p) => markExited(p.id)),
      on<{ key: string; state: AgentState }>("agent-state", (p) => setAgentState(p.key, p.state)),
      on<CrewEvent>("crew", onCrew),
      on<PermissionAsk>("permission-request", (a) => setPermissions((l) => [...l.filter((x) => x.id !== a.id), a])),
      on<ScreenPrompt>("screen-prompt", onScreenPrompt),
    ]);
    await subscribeFrames(pushFrame);
    for (const p of await api.list()) addPane(p);
    await refreshProjects();
    setPermissions(await api.permissions().catch(() => []));
    setScreenPrompts(await api.screenPrompts().catch(() => []));
    const poll = setInterval(async () => setStats(await api.stats().catch(() => null)), 3000);
    onCleanup(() => {
      clearInterval(poll);
      offs.forEach((off) => off());
    });
  });

  return (
    <div class="shell" classList={{ "no-side": !sidebarOpen(), "no-panel": !panelOpen() }}>
      <TopBar />
      <Show when={sidebarOpen()}>
        <Sidebar />
      </Show>
      <main class="center">
        <div class="view">
        <Switch>
          <Match when={view() === "chat"}>
            <Conversation />
          </Match>
          <Match when={view() === "run"}>
            <RunView />
          </Match>
          <Match when={view() === "panes"}>
            <PaneGrid />
          </Match>
          <Match when={view() === "os"}>
            <MethodsView />
          </Match>
          <Match when={view() === "agents"}>
            <LookoutView />
          </Match>
        </Switch>
        </div>
        <TermDock />
      </main>
      <Show when={panelOpen()}>
        <RightPanel />
      </Show>
      <Palette />
      <ProjectDialog />
      <Toasts />
    </div>
  );
}
