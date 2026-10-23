// Project actions shared by the sidebar, top bar and palette.

import { open } from "@tauri-apps/plugin-dialog";
import { api } from "../api";
import { projectPanes, setActiveProject, setFocused, setInfo, setProjects, setView } from "../store";
import { setProjectDialog, toast } from "./state";

export async function refreshProjects() {
  const list = await api.projects().catch(() => []);
  setProjects(list);
  const active = list.find((p) => p.active);
  setActiveProject(active?.id ?? null);
  setInfo(await api.info().catch(() => null));
}

/** Native folder picker; falls back to the path dialog if unavailable. */
export async function addProject() {
  let path: string | null = null;
  try {
    const picked = await open({ directory: true, multiple: false, title: "Ajouter un projet" });
    path = typeof picked === "string" ? picked : null;
  } catch {
    setProjectDialog(true);
    return;
  }
  if (!path) return;
  await openPath(path);
}

export async function openPath(path: string) {
  try {
    const rep = await api.openProject(path, false);
    await refreshProjects();
    setView("overview");
    if (rep.written.length) toast({ title: "Projet équipé", body: `${rep.written.length} fichier(s) installé(s) dans ${path}`, tone: "ok" });
  } catch (e) {
    toast({ title: "Ouverture impossible", body: String(e), tone: "danger" });
  }
}

export async function switchProject(id: string) {
  setActiveProject(id);
  await api.activate(id).catch(() => {});
  setInfo(await api.info().catch(() => null));
  setFocused(projectPanes()[0]?.id ?? null);
  setView("overview");
}

export async function closeProject(id: string, name: string) {
  if (!confirm(`Fermer le projet « ${name} » ? Ses terminaux et son run en cours seront arrêtés (rien n'est supprimé).`)) return;
  await api.closeProject(id).catch((e) => toast({ title: "Fermeture impossible", body: String(e), tone: "danger" }));
  await refreshProjects();
}
