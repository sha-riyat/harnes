import { For, Show, createSignal } from "solid-js";
import { activeProject, currentProject, projectPanes, projects, setView, view } from "../store";
import { addProject, switchProject } from "../ui/projects";
import Icon from "../ui/Icon";
import * as I from "../ui/icons";
import { cycleTheme, panelOpen, setPaletteOpen, setPanelOpen, setSidebarOpen, sidebarOpen, theme } from "../ui/state";

export default function TopBar() {
  const [menu, setMenu] = createSignal(false);
  return (
    <header class="topbar">
      <div class="tb-left">
        <button class="icon-btn" classList={{ on: sidebarOpen() }} title="Barre latérale" onClick={() => setSidebarOpen(!sidebarOpen())}>
          <Icon icon={I.SidebarLeft} size={17} />
        </button>
        <div class="prov-menu">
          <button class="project-switch" onClick={() => (projects().length ? setMenu(!menu()) : addProject())} title={currentProject()?.root ?? "Ajouter un projet"}>
            <Icon icon={I.Folder} size={15} />
            <span>{currentProject()?.name ?? "Ajouter un projet"}</span>
            <Show when={projects().length > 1}>
              <span class="count">{projects().length}</span>
            </Show>
          </button>
          <Show when={menu()}>
            <ul class="menu left" onMouseLeave={() => setMenu(false)}>
              <For each={projects()}>
                {(p) => (
                  <li classList={{ on: p.id === activeProject() }} onClick={() => { setMenu(false); switchProject(p.id); }}>
                    {p.name} <span class="muted small">{p.running ? "· run en cours" : ""}</span>
                  </li>
                )}
              </For>
              <li class="sep" onClick={() => { setMenu(false); addProject(); }}>+ Ajouter un projet</li>
            </ul>
          </Show>
        </div>
      </div>

      <nav class="seg" aria-label="Vue">
        <button classList={{ on: view() === "panes" }} onClick={() => setView("panes")}>
          <Icon icon={I.Grid} size={14} /> Panes
          <Show when={projectPanes().length}>
            <span class="count">{projectPanes().length}</span>
          </Show>
        </button>
        <button classList={{ on: view() === "agents" }} onClick={() => setView("agents")}>
          <Icon icon={I.Dashboard} size={14} /> Tous les agents
        </button>
      </nav>

      <div class="tb-right">
        <button class="search" onClick={() => setPaletteOpen(true)}>
          <Icon icon={I.Search} size={14} />
          <span>Rechercher une action</span>
          <kbd>Ctrl K</kbd>
        </button>
        <button class="icon-btn" title={`Thème : ${theme()}`} onClick={cycleTheme}>
          <Icon icon={theme() === "light" ? I.Sun : theme() === "dark" ? I.Moon : I.Settings} size={16} />
        </button>
        <button class="icon-btn" classList={{ on: panelOpen() }} title="Panneau de l'équipage" onClick={() => setPanelOpen(!panelOpen())}>
          <Icon icon={I.SidebarRight} size={17} />
        </button>
      </div>
    </header>
  );
}
