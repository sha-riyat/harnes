// The run's record (`harnes.view.v1`) and the human decisions on it. Everything here
// comes from files or telemetry events, never from generated markup: the gate, the
// reviews, the plan, memory to read, decisions, findings, the trace, the harness,
// worktrees and the evaluation.

import { For, Show, createMemo, createResource, createSignal, type JSX } from "solid-js";
import { api, type EvalResult, type EvalSuite, type MemoryCandidate, type TraceItem } from "../../api";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { cx } from "@/lib/cva";
import { refetchRun, runView } from "../../model";
import { activeProject } from "../../store";
import Icon from "../../ui/Icon";
import { ic } from "../../ui/ph";
import { toast } from "../../ui/state";

const time = (ms: number) => (ms ? new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" }) : "–");
const day = (ms: number) => (ms ? new Date(ms).toLocaleDateString([], { month: "short", day: "numeric" }) : "–");
const n = (v?: number) => (v === undefined ? "–" : v >= 1_000_000 ? `${(v / 1_000_000).toFixed(1)}M` : v >= 1_000 ? `${(v / 1_000).toFixed(1)}k` : `${v}`);

function Section(props: { title: string; meta?: string; children: JSX.Element; class?: string }) {
  return (
    <Card class={cx("m-rise gap-0 rounded-xl py-0 shadow-none", props.class)}>
      <header class="flex items-center gap-2 border-b px-4 py-2.5"><h2 class="text-sm font-semibold">{props.title}</h2><Show when={props.meta}><span class="num text-xs text-muted-foreground">{props.meta}</span></Show></header>
      <div class="flex flex-col gap-3 px-4 py-3.5 text-sm">{props.children}</div>
    </Card>
  );
}
const Mono = (p: { children: JSX.Element }) => <code class="rounded bg-muted px-1.5 py-0.5 font-mono text-[12.5px] break-all">{p.children}</code>;
const Dim = (p: { children: JSX.Element; mono?: boolean }) => <p class={cx("text-muted-foreground", p.mono && "font-mono text-xs break-all")}>{p.children}</p>;
const Reason = (p: { value: () => string; set: (v: string) => void; label: string }) => (
  <input class="h-8 min-w-[220px] flex-1 rounded-lg border bg-transparent px-3 text-sm outline-none focus-visible:border-foreground/30" placeholder={p.label} aria-label={p.label} value={p.value()} onInput={(e) => p.set(e.currentTarget.value)} />
);

function Trace(props: { item: TraceItem }) {
  const bad = () => props.item.status === "error" || props.item.status === "denied";
  return (
    <li class="flex items-baseline gap-3 border-b px-4 py-2 text-[13px] last:border-b-0">
      <span class="num w-[68px] shrink-0 font-mono text-xs text-muted-foreground">{time(props.item.at)}</span>
      <span class="w-20 shrink-0 text-xs text-muted-foreground">{props.item.kind}</span>
      <span class="min-w-0 flex-1 text-muted-foreground">{props.item.summary}<Show when={props.item.path}> <Mono>{props.item.path}</Mono></Show><Show when={props.item.command}> <Mono>{props.item.command}</Mono></Show></span>
      <Show when={props.item.status}><Badge variant={bad() ? "default" : "outline"} class="font-normal">{props.item.status}</Badge></Show>
    </li>
  );
}

export default function Run() {
  const [memTick, setMemTick] = createSignal(0);
  const [reason, setReason] = createSignal("");
  const [kind, setKind] = createSignal("all");
  const [busy, setBusy] = createSignal<string | null>(null);
  const [evaluation, setEvaluation] = createSignal<EvalResult | null>(null);
  const [tune, setTune] = createSignal<string | null>(null);

  const [memory] = createResource(() => ({ project: activeProject(), tick: memTick() }), async (s) => (s.project ? await api.memoryCandidates("candidate", undefined, 100).catch(() => []) : []));
  const [suites] = createResource(() => activeProject(), async (p) => (p ? await api.evalList().catch(() => [] as EvalSuite[]) : ([] as EvalSuite[])));
  const [harness] = createResource(() => activeProject(), async (p) => (p ? await api.harnessGet().catch(() => null) : null));
  const [worktrees, { refetch: refetchWorktrees }] = createResource(() => activeProject(), async (p) => (p ? await api.worktreeList().catch(() => []) : []));
  const data = runView;

  const kinds = createMemo(() => ["all", ...Array.from(new Set((data()?.trace ?? []).map((t) => t.kind))).sort()]);
  const trace = createMemo(() => (data()?.trace ?? []).filter((t) => kind() === "all" || t.kind === kind()));
  const refresh = () => refetchRun();

  const guard = async (key: string, fn: () => Promise<void>, fail: string) => {
    setBusy(key);
    try { await fn(); } catch (e) { toast({ title: fail, body: String(e), tone: "danger" }); } finally { setBusy(null); }
  };
  const decideMemory = (c: MemoryCandidate, action: "approve" | "reject") =>
    guard(`m${c.id}`, async () => {
      await api.memoryReview(c.id, action, reason());
      toast({ title: action === "approve" ? "Memory approved" : "Candidate rejected", body: c.title, tone: "ok" });
      setMemTick((v) => v + 1);
      refetchRun();
    }, "Review failed");
  const decideRun = (k: "gate" | "review", id: string, action: "approve" | "reject") =>
    guard(`${k}${id}`, async () => {
      const run = data()?.run;
      if (!run) return;
      await api.runDecide(run, k, id, action, reason());
      toast({ title: action === "approve" ? "Decision recorded: approved" : "Decision recorded: rejected", body: id, tone: "ok" });
      refresh();
    }, "Decision failed");
  const runEval = () =>
    guard("eval", async () => {
      const out = await api.evalRun(suites()?.[0]?.name);
      setEvaluation(out.result);
      toast({ title: out.result.passed ? "Evaluation passed" : "Evaluation failed", body: `${out.result.suite} · ${out.result.passed_tasks}/${out.result.total_tasks} tasks`, tone: out.result.passed ? "ok" : "danger" });
    }, "Evaluation failed");
  const planTune = () => guard("tune", async () => { const out = await api.tunePlan(); setTune(out.path); toast({ title: "Tuning plan written", body: out.path, tone: "ok" }); }, "Tuning failed");
  const removeWorktree = (run: string, lot: string) => guard("worktree", async () => { await api.worktreeRemove(run, lot); await refetchWorktrees(); toast({ title: "Worktree removed", body: `${run}/${lot}`, tone: "ok" }); }, "Worktree kept");
  const blocked = (a: string) => busy() !== null && busy() !== a;

  return (
    <section class="flex min-h-0 flex-1 flex-col">
      <header class="flex items-end gap-4 px-6 pt-5 pb-3">
        <div class="min-w-0 flex-1">
          <p class="text-xs text-muted-foreground">Run record</p>
          <h1 class="truncate text-xl font-semibold tracking-tight">{data()?.task || "No run yet"}</h1>
          <Show when={data()}><p class="num text-sm text-muted-foreground">run {data()!.run} · {data()!.phase} · {data()!.files.length} files · {day(data()!.started_at)} {time(data()!.started_at)}</p></Show>
        </div>
        <Button variant="ghost" size="sm" class="text-muted-foreground" onClick={refresh}><Icon icon={ic.arrowsClockwise} size={15} />Refresh</Button>
      </header>

      <Show when={data()} fallback={<p class="m-auto text-sm text-muted-foreground">No run in this project. Start a task from the chat.</p>}>
        {(v) => (
          <div class="mx-auto flex w-full max-w-[960px] min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-6 pt-2 pb-6">
            <Card class="gap-0 rounded-xl py-0 shadow-none">
              <div class="grid grid-cols-6 divide-x">
                <For each={[["Input", n(v().usage.input_tokens)], ["Output", n(v().usage.output_tokens)], ["Cache", n(v().usage.cache_read_tokens)], ["Requests", String(v().usage.requests)], ["Events", String(v().trace.length)], ["Findings", String(v().findings.length)]]}>
                  {([l, x]) => <div class="flex flex-col gap-0.5 px-4 py-3"><p class="text-xs text-muted-foreground">{l}</p><p class="num text-xl tracking-tight">{x}</p></div>}
                </For>
              </div>
            </Card>

            <Show when={v().gate}>
              {(g) => (
                <Section title="Human gate" meta={g().ready ? "ready" : "not ready"}>
                  <p>{g().summary_for_user}</p>
                  <Show when={g().unmet.length}><ul class="flex flex-col gap-1"><For each={g().unmet}>{(u) => <li class="text-muted-foreground"><Mono>{u.lot}</Mono> {u.problem} — {u.fix}</li>}</For></ul></Show>
                  <div class="flex flex-wrap items-center gap-2"><Reason label="Reason (optional)" value={reason} set={setReason} /><Button size="sm" disabled={blocked("gategate")} onClick={() => decideRun("gate", "gate", "approve")}>Approve the gate</Button><Button size="sm" variant="ghost" class="text-muted-foreground" disabled={blocked("gategate")} onClick={() => decideRun("gate", "gate", "reject")}>Reject</Button></div>
                </Section>
              )}
            </Show>

            <Show when={v().reviews.length}>
              <Section title="Reviews" meta={`${v().reviews.length} round${v().reviews.length === 1 ? "" : "s"}`}>
                <For each={v().reviews}>
                  {(r, i) => (
                    <div class="flex flex-col gap-2 rounded-lg border p-3">
                      <div class="flex items-center gap-2"><Badge variant="outline" class="font-normal">{r.verdict}</Badge><span class="text-xs text-muted-foreground">round {i()}</span><span class="flex-1" /><Button size="sm" variant="ghost" class="h-7 text-muted-foreground" disabled={!!busy()} onClick={() => decideRun("review", `review-${i()}`, "approve")}>Keep</Button><Button size="sm" variant="ghost" class="h-7 text-muted-foreground" disabled={!!busy()} onClick={() => decideRun("review", `review-${i()}`, "reject")}>Send back</Button></div>
                      <For each={r.fixes}>{(f) => <p class="text-muted-foreground"><Mono>{f.where}</Mono> {f.problem} → {f.fix}</p>}</For>
                      <For each={r.commits}>{(c) => <p class="flex items-center gap-2"><Icon icon={ic.commit} size={14} class="text-muted-foreground" /><Mono>{c.message}</Mono><span class="text-xs text-muted-foreground">{c.lot} · {c.files.length} file{c.files.length === 1 ? "" : "s"}</span></p>}</For>
                    </div>
                  )}
                </For>
              </Section>
            </Show>

            <Show when={v().plan}>
              {(p) => (
                <Section title="Plan" meta={`${p().lots.length} lots`}>
                  <ol class="flex flex-col gap-3"><For each={p().lots}>{(lot) => <li><p><Mono>{lot.id}</Mono> <strong class="font-medium">{lot.title}</strong></p><Dim>{lot.instructions}</Dim></li>}</For></ol>
                </Section>
              )}
            </Show>

            <Section title="Memory to read" meta={`${memory()?.length ?? 0} candidate${(memory()?.length ?? 0) === 1 ? "" : "s"}`}>
              <Show when={memory()?.length} fallback={<Dim>Nothing to read. The lessons of a run arrive here after the review.</Dim>}>
                <For each={memory()}>
                  {(c) => (
                    <div class="flex flex-col gap-1.5 rounded-lg border p-3">
                      <div class="flex flex-wrap items-center gap-2"><Badge variant="outline" class="font-normal">{c.kind}</Badge><strong class="font-medium">{c.title}</strong><span class="text-xs text-muted-foreground">confidence {Math.round(c.confidence * 100)}%</span><span class="flex-1" /><Button size="sm" variant="ghost" class="h-7 text-muted-foreground" disabled={!!busy()} onClick={() => decideMemory(c, "reject")}>Reject</Button><Button size="sm" class="h-7" disabled={!!busy()} onClick={() => decideMemory(c, "approve")}>Approve</Button></div>
                      <p>{c.body}</p>
                      <Show when={c.applicability}><Dim>When: {c.applicability}</Dim></Show>
                      <Show when={c.evidence.length}><Dim mono>evidence: {c.evidence.join(", ")}</Dim></Show>
                    </div>
                  )}
                </For>
                <Reason label="Reason for the decision (optional)" value={reason} set={setReason} />
              </Show>
            </Section>

            <Show when={v().decisions.length}><Section title="Human decisions" meta={`${v().decisions.length}`}><ul class="flex flex-col gap-1"><For each={v().decisions}>{(d) => <li><Mono>{d.kind}</Mono> {d.id} · {d.action} <span class="text-muted-foreground">{d.reason}</span></li>}</For></ul></Section></Show>
            <Show when={v().findings.length}><Section title="Guard findings" meta={`${v().findings.length}`}><ul class="flex flex-col gap-1.5"><For each={v().findings}>{(f) => <li class="flex items-baseline gap-2"><Badge variant={f.severity === "high" || f.severity === "critical" ? "default" : "outline"} class="font-normal">{f.severity}</Badge>{f.message}</li>}</For></ul></Section></Show>

            <Card class="m-rise gap-0 overflow-hidden rounded-xl py-0 shadow-none">
              <header class="flex items-center gap-2 border-b px-4 py-2.5"><h2 class="text-sm font-semibold">Trace</h2><span class="num text-xs text-muted-foreground">{trace().length}/{v().trace.length}</span><select class="ml-auto h-7 rounded-lg border bg-transparent px-2 text-xs outline-none" aria-label="Filter the trace" value={kind()} onChange={(e) => setKind(e.currentTarget.value)}><For each={kinds()}>{(k) => <option value={k} class="bg-popover">{k}</option>}</For></select></header>
              <ol class="max-h-[360px] overflow-y-auto"><For each={trace()} fallback={<li class="px-4 py-4 text-sm text-muted-foreground">No event for this filter.</li>}>{(e) => <Trace item={e} />}</For></ol>
            </Card>

            <Show when={harness()}>
              {(h) => (
                <Section title="Harness" meta={`${h().spec.provider} · ${h().spec.role} · v${h().spec.version}`}>
                  <ul class="flex flex-col gap-1 text-muted-foreground">
                    <li><Mono>prompt</Mono> {h().spec.components.system_prompt}</li>
                    <li><Mono>guards</Mono> {h().spec.components.guardrails} · <Mono>memory</Mono> {h().spec.components.memory}</li>
                    <li><Mono>tools</Mono> {h().spec.components.tool_guidance} · <Mono>skills</Mono> {h().spec.components.skills.join(", ") || "–"}</li>
                    <li><Mono>subagents</Mono> {h().spec.components.subagents.join(", ") || "–"} · <Mono>scripts</Mono> {h().spec.components.scripts.join(", ") || "–"}</li>
                  </ul>
                  <Dim>budgets: {h().spec.budgets.always_on_tokens} tokens always loaded · {h().spec.budgets.on_demand_tokens} on demand</Dim>
                  <Dim mono>{h().path}</Dim>
                  <div class="flex items-center gap-3"><Button size="sm" variant="outline" disabled={!!busy()} onClick={planTune}>Propose a tuning</Button><Dim>One component at a time, on the smoke suite. The plan is written, not applied.</Dim></div>
                  <Show when={tune()}><Dim mono>plan: {tune()}</Dim></Show>
                </Section>
              )}
            </Show>

            <Show when={(worktrees() ?? []).length > 0}>
              <Section title="Worktrees" meta={`${worktrees()!.length} isolated`}>
                <ul class="flex flex-col gap-2"><For each={worktrees()}>{(w) => <li class="flex flex-wrap items-center gap-2"><Mono>{w.branch}</Mono><span class="text-muted-foreground">{w.lot}</span><span class="min-w-0 flex-1 truncate font-mono text-xs text-muted-foreground">{w.path}</span><Button size="sm" variant="ghost" class="h-7 text-muted-foreground" disabled={!!busy()} onClick={() => removeWorktree(w.base, w.lot)}>Remove</Button></li>}</For></ul>
                <div class="flex items-center gap-3"><Button size="sm" variant="ghost" class="text-muted-foreground" disabled={!!busy()} onClick={() => refetchWorktrees()}>Refresh</Button><Dim>A conflict stays here until a person resolves it; "Remove" refuses a dirty tree.</Dim></div>
              </Section>
            </Show>

            <Section title="Evaluation" meta={(suites() ?? []).map((s) => s.name).join(" · ") || "smoke"}>
              <div class="flex items-center gap-3"><Button size="sm" disabled={!!busy()} onClick={runEval}>Run the suite</Button><Dim>Deterministic checks, no model. Results in .harnes/evals/results.</Dim></div>
              <Show when={evaluation()}>
                {(r) => (
                  <div class="m-rise flex flex-col gap-2 rounded-lg border p-3">
                    <p class="flex items-center gap-2"><Badge variant={r().passed ? "default" : "outline"} class="font-normal">{r().passed ? "pass" : "fail"}</Badge>{r().suite} · {r().passed_tasks}/{r().total_tasks} · {r().duration_ms} ms</p>
                    <For each={r().tasks}>{(t) => <div><p><strong class="font-medium">{t.title}</strong> <span class="text-xs text-muted-foreground">{t.passed ? "ok" : "failed"}</span></p><For each={t.checks}>{(c) => <p class={cx("text-xs", c.passed ? "text-muted-foreground" : "text-foreground")}><Mono>{c.command}</Mono> · {c.duration_ms} ms</p>}</For></div>}</For>
                  </div>
                )}
              </Show>
            </Section>
          </div>
        )}
      </Show>
    </section>
  );
}
