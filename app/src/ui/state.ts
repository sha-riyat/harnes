// Layout and preference state (per viewer; persisted in localStorage when possible).

import { createSignal } from "solid-js";
import { createStore, produce } from "solid-js/store";

function load<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw === null ? fallback : (JSON.parse(raw) as T);
  } catch {
    return fallback;
  }
}

function persisted<T>(key: string, fallback: T) {
  const [get, set] = createSignal<T>(load(key, fallback));
  const save = (v: T) => {
    set(() => v);
    try {
      localStorage.setItem(key, JSON.stringify(v));
    } catch {
      /* storage unavailable: keep in memory */
    }
  };
  return [get, save] as const;
}

export type Theme = "system" | "light" | "dark";
export type PanelTab = "crew" | "threads" | "usage" | "sources";

export const [sidebarOpen, setSidebarOpen] = persisted("harnes.sidebar", true);
/** Height of the terminal dock, in pixels. */
export const [dockHeight, setDockHeight] = persisted("harnes.dockHeight", 300);
export const [panelOpen, setPanelOpen] = persisted("harnes.panel", true);
export const [panelTab, setPanelTab] = persisted<PanelTab>("harnes.panelTab.v2", "threads");
export const [theme, setThemeRaw] = persisted<Theme>("harnes.theme", "system");
export interface Prefs { toastDone: boolean; toastFail: boolean }
/** Which interruptions harnes may raise as a toast, besides the questions that wait for you. */
export const [prefs, setPrefs] = persisted<Prefs>("harnes.prefs", { toastDone: true, toastFail: true });
export const [paletteOpen, setPaletteOpen] = createSignal(false);
/** The settings area shown; the notification panel can send you straight to one. */
export const [settingsSection, setSettingsSection] = createSignal("models");
export const [projectDialog, setProjectDialog] = createSignal(false);

const dark = matchMedia("(prefers-color-scheme: dark)");

/** The look the user asked for, or the system's when they asked for none. */
export const resolvedMode = () => (theme() === "system" ? (dark.matches ? "dark" : "light") : theme());

export function applyTheme(t: Theme = theme()) {
  const root = document.documentElement;
  if (t === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", t);
  root.setAttribute("data-mode", t === "system" ? (dark.matches ? "dark" : "light") : t);
}
dark.addEventListener("change", () => applyTheme());

export function setTheme(t: Theme) {
  setThemeRaw(t);
  applyTheme(t);
}

export function cycleTheme() {
  setTheme(theme() === "system" ? "light" : theme() === "light" ? "dark" : "system");
}

export interface Toast {
  id: number;
  title: string;
  body?: string;
  tone: "info" | "warn" | "ok" | "danger";
  action?: { label: string; run: () => void };
}

export const [toasts, setToasts] = createStore<Toast[]>([]);
let nextToast = 1;

export function toast(t: Omit<Toast, "id">, ms = 6000) {
  const id = nextToast++;
  setToasts(produce((l) => {
    l.push({ ...t, id });
    if (l.length > 4) l.shift();
  }));
  if (ms > 0) setTimeout(() => dismiss(id), ms);
}

export function dismiss(id: number) {
  setToasts((l) => l.filter((t) => t.id !== id));
}
