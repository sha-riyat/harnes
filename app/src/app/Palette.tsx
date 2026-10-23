// Ctrl K: search an action, a project or a terminal. Same actions as before, drawn as
// a centred panel; arrows move, Enter runs, Escape closes.

import { For, Show, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import { api } from "../api";
import { cx } from "@/lib/cva";
import { Kbd } from "@/components/ui/kbd";
import { isRunning } from "../model";
import { activeProject, addPane, info, projectPanes, projects, setFocused, setView } from "../store";
import Icon from "../ui/Icon";
import { ic } from "../ui/ph";
import { addProject, switchProject } from "../ui/projects";
import { cycleTheme, paletteOpen, setPaletteOpen, setProjectDialog, setSidebarOpen, sidebarOpen } from "../ui/state";

interface Action { label: string; hint?: string; icon: string; run: () => void; enabled?: () => boolean }

export default function Palette() {
  const [q, setQ] = createSignal("");
  const [sel, setSel] = createSignal(0);

  const actions = createMemo<Action[]>(() => [
    { label: "New task for the crew", icon: ic.compass, run: () => (setView("chat"), setTimeout(() => document.getElementById("composer-input")?.focus(), 50)), enabled: () => !!info()?.project },
    { label: "Add a project", icon: ic.folderPlus, run: addProject },
    { label: "Open a project by its path", icon: ic.folder, run: () => setProjectDialog(true) },
    { label: "New shell", icon: ic.terminal, run: () => api.spawn("shell").then((p) => (addPane(p), setFocused(p.id), setView("panes"))) },
    { label: "New Claude session", icon: ic.robot, run: () => api.spawn("claude").then((p) => (addPane(p), setFocused(p.id), setView("panes"))), enabled: () => !!info()?.project },
    { label: "Stop the run", icon: ic.stop, run: () => api.crewCancel(), enabled: isRunning },
    { label: "Overview", icon: ic.grid, run: () => setView("overview") },
    { label: "Chat with the orchestrator", icon: ic.chat, run: () => setView("chat") },
    { label: "Run record", icon: ic.listChecks, run: () => setView("run") },
    { label: "Crew and machine", icon: ic.robot, run: () => setView("agents") },
    { label: "Terminals", icon: ic.terminal, run: () => setView("panes") },
    { label: "Methods (OS packs)", icon: ic.flask, run: () => setView("os") },
    { label: "Settings", icon: ic.gear, run: () => setView("settings") },
    { label: sidebarOpen() ? "Hide the sidebar" : "Show the sidebar", icon: ic.sidebarSimple, run: () => setSidebarOpen(!sidebarOpen()) },
    { label: "Change the theme", icon: ic.palette, run: cycleTheme },
    ...projects().filter((p) => p.id !== activeProject()).map((p) => ({ label: `Project: ${p.name}`, hint: p.running ? "run going" : undefined, icon: ic.folder, run: () => switchProject(p.id) })),
    ...projectPanes().map((p) => ({ label: `Go to ${p.title.replace(/^\d+-/, "")}`, hint: p.role ?? p.program, icon: p.key ? ic.robot : ic.terminal, run: () => (setFocused(p.id), setView("panes")) })),
  ]);
  const shown = createMemo(() => {
    const t = q().toLowerCase().trim();
    return actions().filter((a) => (a.enabled?.() ?? true) && (!t || a.label.toLowerCase().includes(t)));
  });
  const close = () => (setPaletteOpen(false), setQ(""), setSel(0));
  const exec = (a?: Action) => {
    if (!a) return;
    close();
    a.run();
  };

  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") (e.preventDefault(), setPaletteOpen(!paletteOpen()));
    };
    window.addEventListener("keydown", onKey, true);
    onCleanup(() => window.removeEventListener("keydown", onKey, true));
  });

  return (
    <Show when={paletteOpen()}>
      <div class="fixed inset-0 z-50 grid place-items-start justify-center bg-black/50 pt-[14vh]" onMouseDown={close}>
        <div class="m-menu w-[560px] max-w-[92vw] overflow-hidden rounded-2xl border bg-popover text-popover-foreground shadow-2xl" role="dialog" aria-label="Actions" onMouseDown={(e) => e.stopPropagation()}>
          <div class="flex h-12 items-center gap-3 border-b px-4">
            <Icon icon={ic.search} size={17} class="text-muted-foreground" />
            <input
              class="min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
              ref={(el) => setTimeout(() => el.focus())}
              value={q()}
              placeholder="Search an action, a project or a terminal…"
              aria-label="Search"
              onInput={(e) => (setQ(e.currentTarget.value), setSel(0))}
              onKeyDown={(e) => {
                if (e.key === "Escape") close();
                else if (e.key === "ArrowDown") (e.preventDefault(), setSel(Math.min(sel() + 1, shown().length - 1)));
                else if (e.key === "ArrowUp") (e.preventDefault(), setSel(Math.max(sel() - 1, 0)));
                else if (e.key === "Enter") exec(shown()[sel()]);
              }}
            />
            <Kbd>Esc</Kbd>
          </div>
          <ul class="max-h-[340px] overflow-y-auto p-1.5">
            <For each={shown()} fallback={<li class="px-3 py-6 text-center text-sm text-muted-foreground">No action</li>}>
              {(a, i) => (
                <li class={cx("flex h-9 cursor-default items-center gap-3 rounded-lg px-3 text-sm transition-colors", i() === sel() && "bg-accent")} onMouseEnter={() => setSel(i())} onClick={() => exec(a)}>
                  <Icon icon={a.icon} size={16} class="text-muted-foreground" />
                  <span class="flex-1">{a.label}</span>
                  <Show when={a.hint}><span class="text-xs text-muted-foreground">{a.hint}</span></Show>
                </li>
              )}
            </For>
          </ul>
        </div>
      </div>
    </Show>
  );
}
