import { Show, createSignal } from "solid-js";
import { api } from "../api";
import { crew, info, setCrew, setFocused } from "../store";
import Icon from "../ui/Icon";
import * as I from "../ui/icons";
import { setProjectDialog, toast } from "../ui/state";

export const RUNNING = ["planning", "calibrating", "working", "reviewing", "gating", "fixing"];
export const isRunning = () => RUNNING.includes(crew.phase);

/** Bottom prompt box: talk to the orchestrator, who runs the crew. */
export default function Composer() {
  const [task, setTask] = createSignal("");
  let ta!: HTMLTextAreaElement;

  const grow = () => {
    ta.style.height = "auto";
    ta.style.height = `${Math.min(ta.scrollHeight, 220)}px`;
  };

  const start = async () => {
    if (!info()?.project) return setProjectDialog(true);
    if (!task().trim()) return;
    if (!isRunning()) setCrew({ log: [], review: null, gate: null });
    try {
      const pane = await api.say(task());
      setFocused(pane);
      setTask("");
      grow();
    } catch (e) {
      toast({ title: "Message non envoyé", body: String(e), tone: "danger" });
    }
  };

  return (
    <div class="composer" classList={{ busy: isRunning() }}>
      <textarea
        id="composer-input"
        ref={ta}
        rows={1}
        value={task()}
        placeholder={info()?.project ? (isRunning() ? "Parle à l'orchestrateur pendant le run…" : "Parle à l'orchestrateur : décris ce que l'équipage doit faire…") : "Ouvre d'abord un projet"}
        onInput={(e) => {
          setTask(e.currentTarget.value);
          grow();
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            start();
          }
        }}
      />
      <div class="composer-bar">
        <div class="chips">
          <span class="chip accent" title="Ton interlocuteur : il lance et supervise l'équipage">
            <Icon icon={I.Brain} size={13} /> Orchestrateur
          </span>
          <span class="chip" title="Planner, Teacher et Reviewer">
            <Icon icon={I.Brain} size={13} /> Opus
          </span>
          <span class="chip" title="Workers">
            <Icon icon={I.Robot} size={13} /> Sonnet
          </span>
          <span class="chip" title="Scouts, missions rapides">
            <Icon icon={I.Flash} size={13} /> Haiku
          </span>
          <span class="chip" title="Tests e2e avant revue">
            <Icon icon={I.TestTube} size={13} /> e2e
          </span>
        </div>
        <span class="hint muted small">
          <kbd>Entrée</kbd> envoyer · <kbd>Maj Entrée</kbd> ligne
        </span>
        <Show when={isRunning()}>
          <button class="send stop" onClick={() => api.crewCancel()} title="Arrêter le run">
            <Icon icon={I.Stop} size={16} />
          </button>
        </Show>
        <button class="send" disabled={!task().trim()} onClick={start} title="Envoyer à l'orchestrateur">
          <Icon icon={I.Send} size={17} stroke={2} />
        </button>
      </div>
    </div>
  );
}
