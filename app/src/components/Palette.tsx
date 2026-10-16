import { For, Show, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import { api } from "../api";
import { activeProject, addPane, info, projectPanes, projects, setFocused, setView } from "../store";
import { addProject, switchProject } from "../ui/projects";
import Icon, { type IconData } from "../ui/Icon";
import * as I from "../ui/icons";
import { cycleTheme, paletteOpen, panelOpen, setPaletteOpen, setPanelOpen, setProjectDialog, setSidebarOpen, sidebarOpen } from "../ui/state";
import { isRunning } from "./Composer";

interface Action {
  label: string;
  hint?: string;
  icon: IconData;
  run: () => void;
  enabled?: () => boolean;
}

/** Ctrl+K command palette. */
export default function Palette() {
  const [q, setQ] = createSignal("");
  const [sel, setSel] = createSignal(0);

  const actions = createMemo<Action[]>(() => [
    { label: "Nouvelle tâche pour l'équipage", icon: I.Sparkles, run: () => document.getElementById("composer-input")?.focus(), enabled: () => !!info()?.project },
    { label: "Ajouter un projet", icon: I.Folder, run: addProject },
    { label: "Ouvrir un projet par son chemin", icon: I.Folder, run: () => setProjectDialog(true) },
    { label: "Nouveau shell", icon: I.Terminal, run: () => api.spawn("shell").then((p) => { addPane(p); setFocused(p.id); setView("panes"); }) },
    { label: "Nouvelle session Claude", icon: I.Robot, run: () => api.spawn("claude").then((p) => { addPane(p); setFocused(p.id); setView("panes"); }), enabled: () => !!info()?.project },
    { label: "Arrêter le run", icon: I.Stop, run: () => api.crewCancel(), enabled: isRunning },
    { label: "Voir les panes", icon: I.Grid, run: () => setView("panes") },
    { label: "Voir tous les agents", icon: I.Dashboard, run: () => setView("agents") },
    { label: sidebarOpen() ? "Masquer la barre latérale" : "Afficher la barre latérale", icon: I.SidebarLeft, run: () => setSidebarOpen(!sidebarOpen()) },
    { label: panelOpen() ? "Masquer le panneau" : "Afficher le panneau", icon: I.SidebarRight, run: () => setPanelOpen(!panelOpen()) },
    { label: "Changer de thème", icon: I.Moon, run: cycleTheme },
    ...projects().filter((p) => p.id !== activeProject()).map((p) => ({ label: `Projet : ${p.name}`, hint: p.running ? "run en cours" : undefined, icon: I.Folder, run: () => switchProject(p.id) })),
    ...projectPanes().map((p) => ({ label: `Aller à ${p.title}`, hint: p.role ?? p.program, icon: p.key ? I.Robot : I.Terminal, run: () => { setFocused(p.id); setView("panes"); } })),
  ]);

  const shown = createMemo(() => {
    const t = q().toLowerCase().trim();
    return actions().filter((a) => (a.enabled?.() ?? true) && (!t || a.label.toLowerCase().includes(t)));
  });

  const close = () => {
    setPaletteOpen(false);
    setQ("");
    setSel(0);
  };
  const exec = (a?: Action) => {
    if (!a) return;
    close();
    a.run();
  };

  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPaletteOpen(!paletteOpen());
      }
    };
    window.addEventListener("keydown", onKey, true);
    onCleanup(() => window.removeEventListener("keydown", onKey, true));
  });

  return (
    <Show when={paletteOpen()}>
      <div class="overlay" onMouseDown={close}>
        <div class="palette" role="dialog" aria-label="Actions" onMouseDown={(e) => e.stopPropagation()}>
          <div class="pal-input">
            <Icon icon={I.Search} size={16} class="muted" />
            <input
              autofocus
              ref={(el) => setTimeout(() => el.focus())}
              value={q()}
              placeholder="Rechercher une action ou un pane…"
              onInput={(e) => { setQ(e.currentTarget.value); setSel(0); }}
              onKeyDown={(e) => {
                if (e.key === "Escape") close();
                else if (e.key === "ArrowDown") { e.preventDefault(); setSel(Math.min(sel() + 1, shown().length - 1)); }
                else if (e.key === "ArrowUp") { e.preventDefault(); setSel(Math.max(sel() - 1, 0)); }
                else if (e.key === "Enter") exec(shown()[sel()]);
              }}
            />
            <kbd>Échap</kbd>
          </div>
          <ul class="pal-list">
            <For each={shown()} fallback={<li class="muted small pal-empty">Aucune action</li>}>
              {(a, i) => (
                <li classList={{ on: i() === sel() }} onMouseEnter={() => setSel(i())} onClick={() => exec(a)}>
                  <Icon icon={a.icon} size={15} />
                  <span>{a.label}</span>
                  <Show when={a.hint}><span class="muted small">{a.hint}</span></Show>
                </li>
              )}
            </For>
          </ul>
        </div>
      </div>
    </Show>
  );
}
