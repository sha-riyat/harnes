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
export const [panelOpen, setPanelOpen] = persisted("harnes.panel", true);
export const [panelTab, setPanelTab] = persisted<PanelTab>("harnes.panelTab", "crew");
export const [theme, setThemeRaw] = persisted<Theme>("harnes.theme", "system");
export const [paletteOpen, setPaletteOpen] = createSignal(false);
export const [projectDialog, setProjectDialog] = createSignal(false);

export function applyTheme(t: Theme = theme()) {
  if (t === "system") document.documentElement.removeAttribute("data-theme");
  else document.documentElement.setAttribute("data-theme", t);
}

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
