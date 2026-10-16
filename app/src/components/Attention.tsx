// Everything waiting for the user, as cards above the composer:
// permission requests (from the hook) and questions read from agent screens.

import { For, Show, createMemo } from "solid-js";
import { api } from "../api";
import { activeProject, panes, permissions, screenPrompts, setDrawerPane, setPermissions, setScreenPrompts } from "../store";
import Icon from "../ui/Icon";
import { icons as D } from "../ui/icons";
import { toast } from "../ui/state";

const short = (key: string) => key.replace(/^\d+-/, "").replace(/^orch-.*/, "orchestrateur");

export default function Attention() {
  // Permission requests carry the agent key; keep those of the active project.
  const asks = createMemo(() => permissions().filter((a) => {
    const pane = panes.find((p) => p.key === a.agent);
    return !pane?.project || pane.project === activeProject();
  }));
  const questions = createMemo(() => screenPrompts().filter((q) => q.prompt && q.project === activeProject()));

  const decide = async (id: string, d: "allow" | "always" | "deny") => {
    setPermissions((l) => l.filter((a) => a.id !== id));
    await api.answerPermission(id, d).catch((e) => toast({ title: "Réponse non transmise", body: String(e), tone: "danger" }));
  };
  const choose = async (pane: string, keys: string) => {
    setScreenPrompts((l) => l.filter((q) => q.pane !== pane));
    await api.answerScreen(pane, keys).catch((e) => toast({ title: "Réponse non transmise", body: String(e), tone: "danger" }));
  };

  return (
    <Show when={asks().length || questions().length}>
      <div class="attention">
        <For each={asks()}>
          {(a) => (
            <div class="att-card">
              <div class="att-head">
                <Icon icon={D.lock} size={16} />
                <span class="label">autorisation · {short(a.agent)}</span>
                <span class="spacer" />
                <span class="label">{a.tool}</span>
              </div>
              <code class="att-what">{a.summary}</code>
              <div class="att-actions">
                <button class="btn solid" onClick={() => decide(a.id, "allow")}>Autoriser</button>
                <Show when={a.can_remember}>
                  <button class="btn" onClick={() => decide(a.id, "always")}>Toujours</button>
                </Show>
                <button class="btn ghost" onClick={() => decide(a.id, "deny")}>Refuser</button>
              </div>
            </div>
          )}
        </For>
        <For each={questions()}>
          {(q) => (
            <div class="att-card">
              <div class="att-head">
                <Icon icon={D.question} size={16} />
                <span class="label">question · {short(q.agent)}</span>
                <span class="spacer" />
                <button class="icon-btn sm" title="Voir le terminal" onClick={() => setDrawerPane(q.pane)}>
                  <Icon icon={D.terminal} size={15} />
                </button>
              </div>
              <p class="att-q">{q.prompt!.question}</p>
              <div class="att-actions wrap">
                <For each={q.prompt!.choices}>
                  {(c, i) => (
                    <button class="btn" classList={{ solid: i() === 0 }} onClick={() => choose(q.pane, c.keys)}>
                      {c.label}
                    </button>
                  )}
                </For>
              </div>
            </div>
          )}
        </For>
      </div>
    </Show>
  );
}
