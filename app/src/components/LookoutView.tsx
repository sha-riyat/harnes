import { For, Show, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import { api, on, type LookSession, type Lookout } from "../api";
import { panes, setFocused, setView } from "../store";
import Icon from "../ui/Icon";
import { StateMark } from "../ui/ascii/Ascii";
import { icons as D } from "../ui/icons";

const LABEL: Record<string, string> = {
  "needs-you": "te concerne",
  working: "au travail",
  idle: "en attente",
  finished: "terminé",
  failed: "échec",
  unknown: "inconnu",
};

function ago(ms: number): string {
  if (!ms) return "–";
  const s = Math.max(0, Math.round((Date.now() - ms) / 1000));
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.round(s / 60)} min`;
  return `${Math.round(s / 3600)} h`;
}

/** Last-hour sparkline: working (teal) and needs-you (amber). */
function Spark(props: { points: Lookout["history"] }) {
  const W = 260;
  const H = 48;
  const path = (key: "working" | "needs_you") => {
    const pts = props.points;
    if (pts.length < 2) return "";
    const max = Math.max(1, ...pts.map((p) => p.total));
    return pts.map((p, i) => `${((i / (pts.length - 1)) * W).toFixed(1)},${(H - (p[key] / max) * (H - 4) - 2).toFixed(1)}`).join(" ");
  };
  return (
    <svg class="spark" viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" role="img" aria-label="Activité de la dernière heure">
      <polyline points={path("working")} fill="none" stroke="var(--accent)" stroke-width="1.5" />
      <polyline points={path("needs_you")} fill="none" stroke="var(--warn)" stroke-width="1.5" />
    </svg>
  );
}

export default function LookoutView() {
  const [data, setData] = createSignal<Lookout | null>(null);
  const refresh = () => api.lookout().then(setData).catch(() => {});

  onMount(async () => {
    refresh();
    const off = await on("lookout-changed", refresh);
    const t = setInterval(refresh, 5000);
    onCleanup(() => {
      off();
      clearInterval(t);
    });
  });

  const sessions = () => data()?.snapshot.sessions ?? [];
  const needsYou = createMemo(() => sessions().filter((s) => s.status === "needs-you"));
  const paneOf = (s: LookSession) => panes.find((p) => (s.pid && p.pid === s.pid) || (s.harnes_key && p.key === s.harnes_key));

  const jump = (s: LookSession) => {
    const p = paneOf(s);
    if (p) {
      setFocused(p.id);
      setView("panes");
    }
  };

  return (
    <main class="lookout">
      <Show when={needsYou().length}>
        <section class="needs">
          <h3><Icon icon={D.alert} size={13} /> Te concernent ({needsYou().length})</h3>
          <For each={needsYou()}>
            {(s) => (
              <button class="need" onClick={() => jump(s)} title={paneOf(s) ? "Aller au pane" : s.cwd}>
                <StateMark state="blocked" />
                <b>{s.name}</b>
                <span class="muted">{s.project} · {s.provider}</span>
                <span class="why">{s.waiting_text ?? s.waiting_reason ?? "attend une réponse"}</span>
                <span class="muted small">depuis {ago(s.status_since)}</span>
              </button>
            )}
          </For>
        </section>
      </Show>

      <section class="overview">
        <div>
          <h3><Icon icon={D.usage} size={13} /> Dernière heure</h3>
          <Spark points={data()?.history ?? []} />
          <p class="muted small">
            <span style={{ color: "var(--accent)" }}>■</span> au travail · <span style={{ color: "var(--warn)" }}>■</span> te concernent
          </p>
        </div>
        <div class="sources">
          <h3><Icon icon={D.machine} size={13} /> Sources</h3>
          <For each={data()?.snapshot.sources ?? []}>
            {(src) => (
              <div class={`src ${src.state}`} title={src.error ?? ""}>
                {src.source} <span class="muted small">{src.state === "ok" ? src.sessions : src.state}</span>
              </div>
            )}
          </For>
        </div>
      </section>

      <section>
        <h3><Icon icon={D.agent} size={13} /> Sessions ({sessions().length})</h3>
        <table class="sessions">
          <thead>
            <tr><th /><th>Agent</th><th>Fournisseur</th><th>Projet</th><th>Statut</th><th>Depuis</th><th>Tokens</th></tr>
          </thead>
          <tbody>
            <For each={sessions()} fallback={<tr><td colSpan={7} class="muted">Aucune session détectée</td></tr>}>
              {(s) => (
                <tr classList={{ harnes: !!paneOf(s) }} onClick={() => jump(s)}>
                  <td><StateMark state={s.status === "needs-you" ? "blocked" : s.status} id={s.id} /></td>
                  <td>{s.name}<Show when={paneOf(s)}><span class="badge">harnes</span></Show></td>
                  <td><span class={`prov ${s.provider}`}>{s.provider}</span></td>
                  <td class="muted" title={s.cwd}>{s.project}</td>
                  <td>{LABEL[s.status]}</td>
                  <td class="muted">{ago(s.status_since)}</td>
                  <td class="muted">{s.tokens ? `${s.tokens.input + s.tokens.cached} / ${s.tokens.output}` : "–"}</td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </section>

      <section>
        <h3><Icon icon={D.thread} size={13} /> Événements</h3>
        <ol class="events">
          <For each={data()?.events ?? []} fallback={<li class="muted">Rien pour l'instant</li>}>
            {(e) => (
              <li>
                <span class="muted small">{new Date(e.at).toLocaleTimeString()}</span> <b>{e.session_name}</b>{" "}
                {e.kind === "appeared" ? "apparaît" : e.kind === "ended" ? "se termine" : `${LABEL[e.from ?? "unknown"]} → ${LABEL[e.to ?? "unknown"]}`}
              </li>
            )}
          </For>
        </ol>
      </section>
    </main>
  );
}
