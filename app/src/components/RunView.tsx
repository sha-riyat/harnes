// Typed run view (`harnes.view.v1`) and human-in-the-loop decisions.
// Everything here comes from files or telemetry events, never generated HTML.

import { For, Show, createMemo, createResource, createSignal, type JSX } from "solid-js";
import {
  api,
  type EvalResult,
  type EvalSuite,
  type MemoryCandidate,
  type TraceItem,
} from "../api";
import { activeProject, crew } from "../store";
import { toast } from "../ui/state";

const time = (ms: number) => (ms ? new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" }) : "–");
const day = (ms: number) => (ms ? new Date(ms).toLocaleDateString([], { month: "short", day: "numeric" }) : "–");
const n = (v?: number) => (v === undefined ? "–" : v >= 1_000_000 ? `${(v / 1_000_000).toFixed(1)}M` : v >= 1_000 ? `${(v / 1_000).toFixed(1)}k` : `${v}`);

function Card(props: { title: string; meta?: string; children: JSX.Element }) {
  return (
    <article class="run-card">
      <header class="run-card-head">
        <span class="label">{props.title}</span>
        <Show when={props.meta}><span class="label faint">{props.meta}</span></Show>
      </header>
      {props.children}
    </article>
  );
}

function Trace(props: { item: TraceItem }) {
  return (
    <li classList={{ err: props.item.status === "error" || props.item.status === "denied" }}>
      <span class="trace-time">{time(props.item.at)}</span>
      <span class="trace-kind">{props.item.kind}</span>
      <span class="trace-what">
        <span class="dim">{props.item.summary}</span>
        <Show when={props.item.path}><code>{props.item.path}</code></Show>
        <Show when={props.item.command}><code class="cmd">{props.item.command}</code></Show>
      </span>
      <Show when={props.item.status}><span class="badge">{props.item.status}</span></Show>
    </li>
  );
}

export default function RunView() {
  const [tick, setTick] = createSignal(0);
  const [memTick, setMemTick] = createSignal(0);
  const [reason, setReason] = createSignal("");
  const [kind, setKind] = createSignal("all");
  const [busy, setBusy] = createSignal<string | null>(null);
  const [evaluation, setEvaluation] = createSignal<EvalResult | null>(null);
  const [tune, setTune] = createSignal<string | null>(null);

  // `crew` is a reactive proxy; its phase/log length make the view follow a run.
  const [data, { refetch }] = createResource(
    () => ({ project: activeProject(), phase: crew.phase, log: crew.log.length, tick: tick() }),
    async (src) => (src.project ? api.runView().catch(() => null) : null),
  );
  const [memory] = createResource(
    () => ({ project: activeProject(), tick: memTick() }),
    async (src) => (src.project ? api.memoryCandidates("candidate", undefined, 100).catch(() => []) : []),
  );
  const [suites] = createResource(
    () => activeProject(),
    async (p) => (p ? api.evalList().catch(() => [] as EvalSuite[]) : ([] as EvalSuite[])),
  );
  const [harness] = createResource(
    () => activeProject(),
    async (p) => (p ? api.harnessGet().catch(() => null) : null),
  );
  const [worktrees, { refetch: refetchWorktrees }] = createResource(
    () => activeProject(),
    async (p) => (p ? api.worktreeList().catch(() => []) : []),
  );

  const kinds = createMemo(() => ["all", ...Array.from(new Set((data()?.trace ?? []).map((t) => t.kind))).sort()]);
  const trace = createMemo(() => {
    const k = kind();
    return (data()?.trace ?? []).filter((t) => k === "all" || t.kind === k);
  });

  async function decideMemory(c: MemoryCandidate, action: "approve" | "reject") {
    setBusy(`m${c.id}`);
    try {
      await api.memoryReview(c.id, action, reason());
      toast({ title: action === "approve" ? "Mémoire approuvée" : "Candidat rejeté", body: c.title, tone: "ok" });
      setMemTick((v) => v + 1);
      refetch();
    } catch (e) {
      toast({ title: "Revue impossible", body: String(e), tone: "danger" });
    } finally {
      setBusy(null);
    }
  }

  async function decideRun(kindName: "gate" | "review", id: string, action: "approve" | "reject") {
    const run = data()?.run;
    if (!run) return;
    setBusy(`${kindName}${id}`);
    try {
      await api.runDecide(run, kindName, id, action, reason());
      toast({ title: action === "approve" ? "Décision enregistrée : approuvé" : "Décision enregistrée : rejeté", body: id, tone: "ok" });
      setTick((v) => v + 1);
    } catch (e) {
      toast({ title: "Décision impossible", body: String(e), tone: "danger" });
    } finally {
      setBusy(null);
    }
  }

  async function runEval() {
    setBusy("eval");
    try {
      const out = await api.evalRun(suites()?.[0]?.name);
      setEvaluation(out.result);
      toast({
        title: out.result.passed ? "Évaluation passée" : "Évaluation échouée",
        body: `${out.result.suite} · ${out.result.passed_tasks}/${out.result.total_tasks} tâches`,
        tone: out.result.passed ? "ok" : "danger",
      });
    } catch (e) {
      toast({ title: "Évaluation impossible", body: String(e), tone: "danger" });
    } finally {
      setBusy(null);
    }
  }

  async function planTune() {
    setBusy("tune");
    try {
      const out = await api.tunePlan();
      setTune(out.path);
      toast({ title: "Plan de réglage écrit", body: out.path, tone: "ok" });
    } catch (e) {
      toast({ title: "Réglage impossible", body: String(e), tone: "danger" });
    } finally {
      setBusy(null);
    }
  }

  async function removeWorktree(run: string, lot: string) {
    setBusy("worktree");
    try {
      await api.worktreeRemove(run, lot);
      await refetchWorktrees();
      toast({ title: "Worktree retiré", body: `${run}/${lot}`, tone: "ok" });
    } catch (e) {
      toast({ title: "Worktree conservé", body: String(e), tone: "danger" });
    } finally {
      setBusy(null);
    }
  }

  const approveBlocked = (a: string) => busy() !== null && busy() !== a;

  return (
    <section class="runview">
      <header class="run-head">
        <div>
          <span class="label">typed run view</span>
          <h1>{data()?.task || "aucun run"}</h1>
          <Show when={data()}>
            <span class="dim">
              run {data()!.run} · {data()!.phase} · {data()!.files.length} fichiers · {day(data()!.started_at)} {time(data()!.started_at)}
            </span>
          </Show>
        </div>
        <span class="spacer" />
        <button class="btn ghost" onClick={() => setTick((v) => v + 1)}>rafraîchir</button>
      </header>

      <Show when={data()} fallback={<p class="run-empty">Aucun run dans ce projet. Lance une tâche depuis le chat.</p>}>
        {(v) => (
          <div class="run-scroll">
            <div class="run-figures">
              <div class="figure"><span class="fig-n">{n(v().usage.input_tokens)}</span><span class="label">input</span></div>
              <div class="figure"><span class="fig-n">{n(v().usage.output_tokens)}</span><span class="label">output</span></div>
              <div class="figure"><span class="fig-n">{n(v().usage.cache_read_tokens)}</span><span class="label">cache</span></div>
              <div class="figure"><span class="fig-n">{v().usage.requests}</span><span class="label">requêtes</span></div>
              <div class="figure"><span class="fig-n">{v().trace.length}</span><span class="label">événements</span></div>
              <div class="figure"><span class="fig-n">{v().findings.length}</span><span class="label">findings</span></div>
            </div>

            <Show when={v().gate}>
              {(g) => (
                <Card title="Porte humaine" meta={g().ready ? "prête" : "non prête"}>
                  <p class="run-summary">{g().summary_for_user}</p>
                  <Show when={g().unmet.length}>
                    <ul class="run-list">
                      <For each={g().unmet}>{(u) => <li><code>{u.lot}</code> {u.problem} — {u.fix}</li>}</For>
                    </ul>
                  </Show>
                  <div class="run-actions">
                    <input class="run-reason" placeholder="raison (optionnelle)" value={reason()} onInput={(e) => setReason(e.currentTarget.value)} />
                    <button class="btn solid" disabled={approveBlocked("gategate")} onClick={() => decideRun("gate", "gate", "approve")}>approuver la porte</button>
                    <button class="btn ghost" disabled={approveBlocked("gategate")} onClick={() => decideRun("gate", "gate", "reject")}>rejeter</button>
                  </div>
                </Card>
              )}
            </Show>

            <Show when={v().reviews.length}>
              <Card title="Revues" meta={`${v().reviews.length} tour(s)`}>
                <For each={v().reviews}>
                  {(r, i) => (
                    <div class="run-review">
                      <div class="run-review-head">
                        <span class="badge">{r.verdict}</span>
                        <span class="label faint">tour {i()}</span>
                        <span class="spacer" />
                        <button class="btn ghost" disabled={!!busy()} onClick={() => decideRun("review", `review-${i()}`, "approve")}>garder</button>
                        <button class="btn ghost" disabled={!!busy()} onClick={() => decideRun("review", `review-${i()}`, "reject")}>renvoyer</button>
                      </div>
                      <For each={r.fixes}>
                        {(f) => <p class="run-fix"><code>{f.where}</code> {f.problem} → {f.fix}</p>}
                      </For>
                      <For each={r.commits}>
                        {(c) => <p class="run-commit"><code>{c.message}</code> <span class="label faint">{c.lot} · {c.files.length} fichier(s)</span></p>}
                      </For>
                    </div>
                  )}
                </For>
              </Card>
            </Show>

            <Show when={v().plan}>
              {(p) => (
                <Card title="Plan" meta={`${p().lots.length} lots`}>
                  <ol class="run-lots">
                    <For each={p().lots}>{(lot) => <li><code>{lot.id}</code> <strong>{lot.title}</strong><p class="dim">{lot.instructions}</p></li>}</For>
                  </ol>
                </Card>
              )}
            </Show>

            <Card title="Mémoire à relire" meta={`${memory()?.length ?? 0} candidat(s)`}>
              <Show when={memory()?.length} fallback={<p class="dim">Rien à relire. Les leçons de run arrivent ici après la revue.</p>}>
                <For each={memory()}>
                  {(c) => (
                    <div class="run-memory">
                      <div class="run-review-head">
                        <span class="badge">{c.kind}</span>
                        <strong>{c.title}</strong>
                        <span class="label faint">confiance {Math.round(c.confidence * 100)}%</span>
                        <span class="spacer" />
                        <button class="btn ghost" disabled={!!busy()} onClick={() => decideMemory(c, "reject")}>rejeter</button>
                        <button class="btn solid" disabled={!!busy()} onClick={() => decideMemory(c, "approve")}>approuver</button>
                      </div>
                      <p>{c.body}</p>
                      <Show when={c.applicability}><p class="dim">Quand : {c.applicability}</p></Show>
                      <Show when={c.evidence.length}><p class="dim mono">preuve : {c.evidence.join(", ")}</p></Show>
                      <Show when={c.source === "review"}><p class="label faint">proposé par la revue · run {c.run}</p></Show>
                    </div>
                  )}
                </For>
                <input class="run-reason" placeholder="raison de la décision (optionnelle)" value={reason()} onInput={(e) => setReason(e.currentTarget.value)} />
              </Show>
            </Card>

            <Show when={v().decisions.length}>
              <Card title="Décisions humaines" meta={`${v().decisions.length}`}>
                <ul class="run-list">
                  <For each={v().decisions}>{(d) => <li><code>{d.kind}</code> {d.id} · {d.action} <span class="dim">{d.reason}</span></li>}</For>
                </ul>
              </Card>
            </Show>

            <Show when={v().findings.length}>
              <Card title="Findings de garde" meta={`${v().findings.length}`}>
                <ul class="run-list">
                  <For each={v().findings}>{(f) => <li classList={{ err: f.severity === "high" || f.severity === "critical" }}><span class="badge">{f.severity}</span> {f.message}</li>}</For>
                </ul>
              </Card>
            </Show>

            <Card title="Trace" meta={`${trace().length}/${v().trace.length}`}>
              <div class="run-actions">
                <select class="run-select" value={kind()} onChange={(e) => setKind(e.currentTarget.value)}>
                  <For each={kinds()}>{(k) => <option value={k}>{k}</option>}</For>
                </select>
              </div>
              <ol class="run-trace">
                <For each={trace()} fallback={<li class="dim">Aucun événement pour ce filtre.</li>}>{(e) => <Trace item={e} />}</For>
              </ol>
            </Card>

            <Show when={harness()}>
              {(h) => (
                <Card title="Harnais" meta={`${h().spec.provider} · ${h().spec.role} · v${h().spec.version}`}>
                  <ul class="run-list">
                    <li><code>prompt</code> {h().spec.components.system_prompt}</li>
                    <li><code>gardes</code> {h().spec.components.guardrails} · <code>mémoire</code> {h().spec.components.memory}</li>
                    <li><code>outils</code> {h().spec.components.tool_guidance} · <code>skills</code> {h().spec.components.skills.join(", ") || "–"}</li>
                    <li><code>subagents</code> {h().spec.components.subagents.join(", ") || "–"} · <code>scripts</code> {h().spec.components.scripts.join(", ") || "–"}</li>
                  </ul>
                  <p class="dim">budgets : {h().spec.budgets.always_on_tokens} tokens toujours chargés · {h().spec.budgets.on_demand_tokens} à la demande</p>
                  <p class="dim mono">{h().path}</p>
                  <div class="run-actions">
                    <button class="btn ghost" disabled={!!busy()} onClick={planTune}>proposer un réglage</button>
                    <span class="dim">Un composant à la fois, sur la suite smoke. Le plan est écrit, pas appliqué.</span>
                  </div>
                  <Show when={tune()}><p class="dim mono">plan : {tune()}</p></Show>
                </Card>
              )}
            </Show>

            <Show when={(worktrees() ?? []).length > 0}>
              <Card title="Worktrees" meta={`${worktrees()!.length} isolé(s)`}>
                <ul class="run-list">
                  <For each={worktrees()}>
                    {(w) => (
                      <li>
                        <code>{w.branch}</code> · {w.lot} · <span class="dim mono">{w.path}</span>{" "}
                        <button class="btn ghost" disabled={!!busy()} onClick={() => removeWorktree(w.base, w.lot)}>
                          retirer
                        </button>
                      </li>
                    )}
                  </For>
                </ul>
                <div class="run-actions">
                  <button class="btn ghost" disabled={!!busy()} onClick={() => refetchWorktrees()}>rafraîchir</button>
                  <span class="dim">Un conflit reste ici jusqu'à résolution humaine ; « retirer » refuse un arbre sale.</span>
                </div>
              </Card>
            </Show>

            <Card title="Évaluation" meta={(suites() ?? []).map((s) => s.name).join(" · ") || "smoke"}>
              <div class="run-actions">
                <button class="btn solid" disabled={!!busy()} onClick={runEval}>lancer la suite</button>
                <span class="dim">Vérifications déterministes, sans modèle. Résultats dans .harnes/evals/results.</span>
              </div>
              <Show when={evaluation()}>
                {(r) => (
                  <div class="run-eval">
                    <p><span class="badge">{r().passed ? "pass" : "fail"}</span> {r().suite} · {r().passed_tasks}/{r().total_tasks} · {r().duration_ms} ms</p>
                    <For each={r().tasks}>
                      {(t) => (
                        <div>
                          <p><strong>{t.title}</strong> <span class="label faint">{t.passed ? "ok" : "échec"}</span></p>
                          <For each={t.checks}>{(c) => <p classList={{ err: !c.passed }}><code>{c.command}</code> · {c.duration_ms} ms</p>}</For>
                        </div>
                      )}
                    </For>
                  </div>
                )}
              </Show>
            </Card>
          </div>
        )}
      </Show>
    </section>
  );
}
