import { For, Show, createSignal } from "solid-js";
import { openPath } from "../ui/projects";
import Icon from "../ui/Icon";
import Ascii from "../ui/ascii/Ascii";
import { thinking } from "../ui/ascii/pieces";
import { icons as D } from "../ui/icons";
import { dismiss, projectDialog, setProjectDialog, toast, toasts } from "../ui/state";

/** Bottom-right notifications. */
export function Toasts() {
  return (
    <div class="toasts" aria-live="polite">
      <For each={toasts}>
        {(t) => (
          <div class={`toast ${t.tone}`}>
            <Icon icon={t.tone === "ok" ? D.check : t.tone === "info" ? D.alert : D.alert} size={16} />
            <div class="t-body">
              <b>{t.title}</b>
              <Show when={t.body}><p>{t.body}</p></Show>
              <Show when={t.action}>
                <button class="btn ghost" onClick={() => { t.action!.run(); dismiss(t.id); }}>{t.action!.label}</button>
              </Show>
            </div>
            <button class="icon-btn" title="Fermer" onClick={() => dismiss(t.id)}>
              <Icon icon={D.close} size={13} />
            </button>
          </div>
        )}
      </For>
    </div>
  );
}

/** "Open project" dialog: path + equip. */
export function ProjectDialog() {
  const [path, setPath] = createSignal((() => { try { return localStorage.getItem("harnes.project") ?? ""; } catch { return ""; } })());
  const [busy, setBusy] = createSignal(false);

  const open = async () => {
    if (!path().trim()) return;
    setBusy(true);
    try {
      try { localStorage.setItem("harnes.project", path().trim()); } catch { /* ignore */ }
      await openPath(path().trim());
      setProjectDialog(false);
    } catch (e) {
      toast({ title: "Ouverture impossible", body: String(e), tone: "danger" });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Show when={projectDialog()}>
      <div class="overlay" onMouseDown={() => setProjectDialog(false)}>
        <div class="dialog" role="dialog" aria-label="Ouvrir un projet" onMouseDown={(e) => e.stopPropagation()}>
          <h3><Icon icon={D.folder} size={18} /> Ouvrir un projet</h3>
          <p class="muted small">harnes y installe les agents, les réglages et les MCP (harnes-ctx, harnes-e2e), puis démarre le proxy.</p>
          <input
            class="field mono"
            value={path()}
            placeholder="C:\chemin\du\projet"
            spellcheck={false}
            ref={(el) => setTimeout(() => el.focus())}
            onInput={(e) => setPath(e.currentTarget.value)}
            onKeyDown={(e) => { if (e.key === "Enter") open(); if (e.key === "Escape") setProjectDialog(false); }}
          />
          <div class="dialog-actions">
            <button class="btn ghost" onClick={() => setProjectDialog(false)}>Annuler</button>
            <button class="btn primary" disabled={busy() || !path().trim()} onClick={open}>
              <Show when={busy()} fallback={<Icon icon={D.check} size={15} />}><Ascii frame={thinking(4)} /></Show>
              Ouvrir et équiper
            </button>
          </div>
        </div>
      </div>
    </Show>
  );
}
