import { onCleanup, onMount } from "solid-js";
import { api, on, subscribeFrames, type AgentState, type AskPending, type CrewEvent, type PaneInfo, type PermissionAsk, type ScreenPrompt } from "./api";
import { activeProject, addPane, markExited, setAsks, pushCrew, pushFrame, removePane, setAgentState, setPermissions, setScreenPrompts, setStats } from "./store";
import { refreshProjects } from "./ui/projects";
import Overlays from "./app/Overlays";
import Palette from "./app/Palette";
import Shell from "./app/Shell";
import { applyTheme, prefs, toast } from "./ui/state";

function onCrew(ev: CrewEvent & { project?: string }) {
  pushCrew(ev);
  const elsewhere = ev.project && ev.project !== activeProject() ? " · other project" : "";
  if (ev.phase === "done" && prefs().toastDone) toast({ title: `Ready for you${elsewhere}`, body: ev.gate?.summary_for_user, tone: "ok" }, 0);
  if (ev.phase === "failed" && prefs().toastFail) toast({ title: `Run stopped${elsewhere}`, body: ev.message, tone: "danger" }, 0);
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
      on<AskPending>("ask-request", (a) => setAsks((l) => [...l.filter((x) => x.id !== a.id), a])),
      on<ScreenPrompt>("screen-prompt", onScreenPrompt),
    ]);
    await subscribeFrames(pushFrame);
    for (const p of await api.list()) addPane(p);
    await refreshProjects();
    setPermissions(await api.permissions().catch(() => []));
    setAsks(await api.asks().catch(() => []));
    setScreenPrompts(await api.screenPrompts().catch(() => []));
    const poll = setInterval(async () => setStats(await api.stats().catch(() => null)), 3000);
    onCleanup(() => {
      clearInterval(poll);
      offs.forEach((off) => off());
    });
  });

  return (
    <>
      <Shell />
      <Palette />
      <Overlays />
    </>
  );
}
