import { For, Show, createSignal } from "solid-js";
import { activeProject, currentProject, projects, setView, view } from "../store";
import Icon from "../ui/Icon";
import { icons as D } from "../ui/icons";
import { addProject, switchProject } from "../ui/projects";
import { cycleTheme, panelOpen, setPaletteOpen, setPanelOpen, setSidebarOpen, sidebarOpen, theme } from "../ui/state";

// Terminals stay out of the way: the grid is reached from the palette.
const VIEWS = [
  { id: "chat", label: "conversation" },
  { id: "os", label: "méthodes" },
  { id: "agents", label: "machine" },
] as const;

export default function TopBar() {
  const [menu, setMenu] = createSignal(false);
  return (
    <header class="topbar">
      <button class="icon-btn" classList={{ on: sidebarOpen() }} title="Barre latérale" onClick={() => setSidebarOpen(!sidebarOpen())}>
        <Icon icon={D.sidebar} size={17} />
      </button>
      <span class="brand">harnes</span>
      <span class="sep">/</span>
      <div class="pop">
        <button class="crumb" onClick={() => (projects().length ? setMenu(!menu()) : addProject())} title={currentProject()?.root}>
          {currentProject()?.name ?? "ajouter un projet"}
          <Icon icon={D.chevron} size={12} />
        </button>
        <Show when={menu()}>
          <ul class="menu" onMouseLeave={() => setMenu(false)}>
            <For each={projects()}>
              {(p) => (
                <li classList={{ on: p.id === activeProject() }} onClick={() => { setMenu(false); switchProject(p.id); }}>
                  <span>{p.name}</span>
                  <Show when={p.running}><span class="label">run</span></Show>
                </li>
              )}
            </For>
            <li class="menu-sep" onClick={() => { setMenu(false); addProject(); }}>
              <Icon icon={D.plus} size={13} /> ajouter un projet
            </li>
          </ul>
        </Show>
      </div>

      <span class="spacer" />

      <nav class="views" aria-label="Vue">
        <For each={VIEWS}>
          {(v) => (
            <button classList={{ on: view() === v.id }} onClick={() => setView(v.id)}>
              {v.label}
            </button>
          )}
        </For>
      </nav>
      <button class="icon-btn" title="Rechercher ou aller à (Ctrl K)" onClick={() => setPaletteOpen(true)}>
        <Icon icon={D.search} size={16} />
      </button>
      <button class="icon-btn" title={`Thème : ${theme()}`} onClick={cycleTheme}>
        <Icon icon={theme() === "light" ? D.sun : D.moon} size={16} />
      </button>
      <button class="icon-btn" classList={{ on: panelOpen() }} title="Panneau" onClick={() => setPanelOpen(!panelOpen())}>
        <Icon icon={D.sidebar} size={17} class="flip" />
      </button>
    </header>
  );
}
