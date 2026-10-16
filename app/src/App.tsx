import { Show, onCleanup, onMount } from "solid-js";
import { api, on, subscribeFrames, type AgentState, type CrewEvent, type PaneInfo } from "./api";
import Composer from "./components/Composer";
import LookoutView from "./components/LookoutView";
import NextStep from "./components/NextStep";
import { ProjectDialog, Toasts } from "./components/Overlays";
import Palette from "./components/Palette";
import PaneGrid from "./components/PaneGrid";
import Progress from "./components/Progress";
import RightPanel from "./components/RightPanel";
import Sidebar from "./components/Sidebar";
import TopBar from "./components/TopBar";
import { activeProject, addPane, markExited, panes, pushCrew, pushFrame, removePane, setAgentState, setFocused, setStats, setView, view } from "./store";
import { refreshProjects } from "./ui/projects";
import { applyTheme, panelOpen, sidebarOpen, toast } from "./ui/state";

function onCrew(ev: CrewEvent & { project?: string }) {
  pushCrew(ev);
  const where = ev.project && ev.project !== activeProject() ? " (autre projet)" : "";
  if (ev.phase === "done") toast({ title: `Prêt pour toi${where}`, body: ev.gate?.summary_for_user, tone: "ok" }, 0);
  if (ev.phase === "failed") toast({ title: `Le run s'est arrêté${where}`, body: ev.message, tone: "danger" }, 0);
}

function onAgentState(p: { key: string; state: AgentState }) {
  setAgentState(p.key, p.state);
  if (p.state !== "blocked") return;
  const pane = panes.find((x) => x.key === p.key);
  toast(
    {
      title: `${pane?.title ?? p.key} t'attend`,
      body: "Permission ou question dans son pane.",
      tone: "warn",
      action: pane ? { label: "Ouvrir le pane", run: () => { setFocused(pane.id); setView("panes"); } } : undefined,
    },
    15000,
  );
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
        if (r.projects) toast({ title: "Session restaurée", body: `${r.projects} projet(s), ${r.panes} pane(s)${r.runs ? `, ${r.runs} run(s) repris` : ""}`, tone: "ok" });
      }),
      on<string>("pane-closed", (id) => removePane(id)),
      on<{ id: string }>("pane-exit", (p) => markExited(p.id)),
      on<{ key: string; state: AgentState }>("agent-state", onAgentState),
      on<CrewEvent>("crew", onCrew),
    ]);
    await subscribeFrames(pushFrame);
    for (const p of await api.list()) addPane(p);
    await refreshProjects();
    const poll = setInterval(async () => setStats(await api.stats().catch(() => null)), 2000);
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
        <Progress />
        <div class="workspace">
          <Show when={view() === "panes"} fallback={<LookoutView />}>
            <PaneGrid />
          </Show>
        </div>
        <div class="bottom">
          <NextStep />
          <Composer />
        </div>
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
