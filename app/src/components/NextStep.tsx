import { Show } from "solid-js";
import { crew, setCrew } from "../store";
import Icon from "../ui/Icon";
import * as I from "../ui/icons";
import { setPanelOpen, setPanelTab } from "../ui/state";

/** Shown once the Teacher lets the work through: the only moment the user is asked to act. */
export default function NextStep() {
  return (
    <Show when={crew.phase === "done" && crew.gate}>
      {(g) => (
        <div class="next-step">
          <div class="ns-head">
            <Icon icon={I.Check} size={18} />
            <b>Prêt pour toi</b>
            <span class="muted small">la barre du Teacher est atteinte</span>
            <span class="spacer" />
            <button class="icon-btn" title="Masquer" onClick={() => setCrew({ phase: "idle" })}>
              <Icon icon={I.Cancel} size={15} />
            </button>
          </div>
          <p>{g().summary_for_user}</p>
          <div class="ns-actions">
            <button class="btn primary" onClick={() => { setPanelTab("crew"); setPanelOpen(true); }}>
              <Icon icon={I.Commit} size={15} /> Voir les {crew.review?.commits.length ?? 0} commits proposés
            </button>
            <span class="muted small">Teste le résultat, puis valide : rien n'est commité sans toi.</span>
          </div>
        </div>
      )}
    </Show>
  );
}
