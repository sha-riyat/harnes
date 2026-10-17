// The account's limits as Anthropic reports them (read by the proxy on every
// response): the 5-hour window and the week, each drawn in characters with a
// mark where a straight line to the reset says we should be. Past that mark,
// or near the end of the 5-hour window, runs start in eco.

import { Show, createResource, onCleanup, onMount } from "solid-js";
import { api } from "../api";
import { stats } from "../store";

const WIDTH = 28;
const FIVE_H = 5 * 3600;
const WEEK = 7 * 86400;

/** `used` against `pace` (both 0..1) as one line of cells. */
function gauge(used: number, pace: number) {
  const u = Math.round(Math.min(1, used) * WIDTH);
  const p = Math.min(WIDTH - 1, Math.round(pace * WIDTH));
  return Array.from({ length: WIDTH }, (_, i) => (i === p ? "|" : i < u ? "#" : "·")).join("");
}

const pct = (n: number) => `${Math.round(n * 100)}%`;
const when = (epoch: number) => new Date(epoch * 1000).toLocaleString([], { weekday: "short", hour: "2-digit", minute: "2-digit" });
function left(epoch: number) {
  const s = Math.max(0, epoch - Date.now() / 1000);
  const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60);
  return h ? `${h}h${String(m).padStart(2, "0")}` : `${m} min`;
}

export default function WeekCard() {
  const [b, { refetch }] = createResource(() => api.budgetGet().catch(() => null));
  onMount(() => {
    const t = setInterval(refetch, 30_000);
    onCleanup(() => clearInterval(t));
  });
  const elapsed = (reset: number, span: number) => Math.min(1, Math.max(0, 1 - (reset - Date.now() / 1000) / span));
  const runs = () => stats()?.runs;
  const share = (a: number, n: number) => (n ? `${Math.round((a / n) * 100)}%` : "–");

  return (
    <>
      <section class="week">
        <header class="week-head">
          <span class="label">limits</span>
          <Show when={b()?.eco}><span class="badge">eco</span></Show>
        </header>
        <Show when={b()?.limits} fallback={<p class="side-empty">Shown after the first agent request of the session.</p>}>
          {(l) => (
            <>
              <p class="week-line"><b>{pct(l().five_hour)}</b> of 5 h · resets in {left(l().five_hour_reset)}</p>
              <pre class="gauge" aria-label="5-hour window against pace">{gauge(l().five_hour, elapsed(l().five_hour_reset, FIVE_H))}</pre>
              <p class="week-line"><b>{pct(l().seven_day)}</b> of the week · resets {when(l().seven_day_reset)}</p>
              <pre class="gauge" aria-label="Week against pace">{gauge(l().seven_day, elapsed(l().seven_day_reset, WEEK))}</pre>
            </>
          )}
        </Show>
      </section>
      <Show when={runs()?.runs}>
        <div class="figures">
          <div class="figure"><span class="fig-n">{runs()!.mean_rounds.toFixed(2)}</span><span class="label">reviews per run</span></div>
          <div class="figure"><span class="fig-n">{share(runs()!.first_pass, runs()!.runs)}</span><span class="label">pass first time</span></div>
          <div class="figure"><span class="fig-n">{runs()!.important}</span><span class="label">blocking findings</span></div>
          <div class="figure"><span class="fig-n">{share(runs()!.resolved.wrong ?? 0, Object.values(runs()!.resolved).reduce((a, x) => a + x, 0))}</span><span class="label">findings wrong</span></div>
        </div>
      </Show>
    </>
  );
}
