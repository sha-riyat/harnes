import { For, Show, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import { api } from "../api";
import { activeProject, addPane, info, projectPanes, projects, setFocused, setView } from "../store";
import { addProject, switchProject } from "../ui/projects";
import Icon from "../ui/Icon";
import { icons as D, type IconData } from "../ui/icons";
import { cycleTheme, paletteOpen, panelOpen, setPaletteOpen, setPanelOpen, setProjectDialog, setSidebarOpen, sidebarOpen } from "../ui/state";
import { crew } from "../store";
const isRunning = () => ["planning", "calibrating", "working", "reviewing", "gating", "fixing"].includes(crew.phase);

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
    { label: "Nouvelle tâche pour l'équipage", icon: D.orchestrator, run: () => document.getElementById("composer-input")?.focus(), enabled: () => !!info()?.project },
    { label: "Ajouter un projet", icon: D.folder, run: addProject },
    { label: "Ouvrir un projet par son chemin", icon: D.folder, run: () => setProjectDialog(true) },
    { label: "Nouveau shell", icon: D.terminal, run: () => api.spawn("shell").then((p) => { addPane(p); setFocused(p.id); setView("panes"); }) },
    { label: "Nouvelle session Claude", icon: D.agent, run: () => api.spawn("claude").then((p) => { addPane(p); setFocused(p.id); setView("panes"); }), enabled: () => !!info()?.project },
    { label: "Arrêter le run", icon: D.stop, run: () => api.crewCancel(), enabled: isRunning },
    { label: "Chat with the orchestrator", icon: D.orchestrator, run: () => setView("chat") },
    { label: "Voir les terminaux", icon: D.grid, run: () => setView("panes") },
    { label: "Méthodes de l'équipage (packs OS)", icon: D.thread, run: () => setView("os") },
    { label: "Voir tous les agents de la machine", icon: D.machine, run: () => setView("agents") },
    { label: sidebarOpen() ? "Masquer la barre latérale" : "Afficher la barre latérale", icon: D.sidebar, run: () => setSidebarOpen(!sidebarOpen()) },
    { label: panelOpen() ? "Masquer le panneau" : "Afficher le panneau", icon: D.sidebar, run: () => setPanelOpen(!panelOpen()) },
    { label: "Changer de thème", icon: D.moon, run: cycleTheme },
    ...projects().filter((p) => p.id !== activeProject()).map((p) => ({ label: `Projet : ${p.name}`, hint: p.running ? "run en cours" : undefined, icon: D.folder, run: () => switchProject(p.id) })),
    ...projectPanes().map((p) => ({ label: `Aller à ${p.title}`, hint: p.role ?? p.program, icon: p.key ? D.agent : D.terminal, run: () => { setFocused(p.id); setView("panes"); } })),
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
            <Icon icon={D.search} size={16} />
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
