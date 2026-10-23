// The cards of the overview, drawn from the run's real state: the lead card (task,
// lots, figures), the orchestrator's work log, the commits of the review, the crew's
// activity and the "ready for you" card at the end of a run.

import { For, Match, Show, Switch, createEffect, createMemo, createSignal } from "solid-js";
import { api } from "../../api";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { cx } from "@/lib/cva";
import { agents, budget, conv, left, lots, needCount, refetchRun, runView, type LotState } from "../../model";
import { crew, setView } from "../../store";
import Icon from "../../ui/Icon";
import AgentIcon from "../../ui/kit/AgentIcon";
import { createTypewriter } from "../../ui/kit/typewriter";
import { ic } from "../../ui/ph";
import { toast } from "../../ui/state";

const clock = (ms?: number) => (ms ? new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) : "");

const PHASE: Record<string, string> = {
  idle: "No run yet", planning: "Planning", calibrating: "Setting the bar", working: "Building", reviewing: "Reviewing", gating: "Checking the gate", fixing: "Fixing", done: "Ready for you", failed: "Stopped", cancelled: "Cancelled",
};

const tone: Record<LotState, string> = { done: "bg-foreground", review: "bg-foreground/60", working: "bg-foreground/35", needs: "bg-signal", planned: "bg-muted-foreground/20" };

/** The run as one strip: a segment per lot, in plan order. */
export function LotStrip(props: { class?: string }) {
  return (
    <Show when={lots().length} fallback={<div class={cx("flex gap-1", props.class)}><div class="h-1.5 flex-1 rounded-full bg-muted-foreground/15" /></div>}>
      <div class={cx("flex gap-1", props.class)} role="img" aria-label="Lots of the run">
        <For each={lots()}>
          {(l) => (
            <div class="min-w-0 flex-1" title={`${l.id} · ${l.title} · ${l.state}`}>
              <div class={cx("m-tone h-1.5 rounded-full", tone[l.state])} />
              <p class="num mt-1.5 truncate text-xs text-muted-foreground">{l.id}</p>
            </div>
          )}
        </For>
      </div>
    </Show>
  );
}

function Cell(props: { label: string; note?: string; signal?: boolean; children: any }) {
  return (
    <div class="flex flex-col gap-0.5 px-5 py-3">
      <p class="text-xs text-muted-foreground">{props.label}</p>
      <p class={cx("num flex items-baseline gap-1 text-[28px] leading-9 tracking-tight", props.signal && "text-signal")}>{props.children}</p>
      <p class="text-xs text-muted-foreground">{props.note ?? " "}</p>
    </div>
  );
}

export function LeadCard() {
  const working = () => agents().filter((a) => a.state === "working").length;
  const done = () => lots().filter((l) => l.state === "done").length;
  const five = () => budget()?.limits;
  return (
    <Card class="gap-0 rounded-lg py-0 shadow-none">
      <div class="flex flex-col gap-2 px-5 py-4">
        <p class="text-xs text-muted-foreground">Current run</p>
        <h2 class="text-base leading-snug font-semibold">{runView()?.task || "Nothing running. Tell the orchestrator what to build."}</h2>
        <p class="num text-sm text-muted-foreground">
          {PHASE[crew.phase] ?? crew.phase}
          <Show when={crew.round > 0}> · round {crew.round}</Show>
          <Show when={runView()?.started_at}> · started {clock(runView()!.started_at)}</Show>
        </p>
        <LotStrip class="mt-1" />
      </div>
      <div class="grid grid-cols-4 divide-x border-t">
        <Cell label="Working" note="agents"><Show when={working()} keyed fallback={<span>0</span>}>{(n) => <span class="m-tick">{n}</span>}</Show><span class="text-sm text-muted-foreground">/ {agents().length}</span></Cell>
        <Cell label="Needs you" note={needCount() ? "answer below" : "all answered"} signal={needCount() > 0}><Show when={needCount()} keyed fallback={<span>0</span>}>{(n) => <span class="m-tick">{n}</span>}</Show></Cell>
        <Cell label="Lots done" note={lots().length ? "of the plan" : "no plan yet"}><Show when={done()} keyed fallback={<span>0</span>}>{(n) => <span class="m-tick">{n}</span>}</Show><span class="text-sm text-muted-foreground">/ {lots().length}</span></Cell>
        <Cell label="5-hour usage" note={five() ? `resets in ${left(five()!.five_hour_reset)}` : "after the first request"}><span>{five() ? Math.round(five()!.five_hour * 100) : "–"}</span><Show when={five()}><span class="text-sm text-muted-foreground">%</span></Show></Cell>
      </div>
    </Card>
  );
}

const toolIcon = (tool = "") => (/read|grep|glob|search|thread/i.test(tool) ? ic.file : /edit|write|patch/i.test(tool) ? ic.pencil : /bash|run|shell|test/i.test(tool) ? ic.terminal : ic.lightning);

/** The orchestrator's latest tool calls, the last one breathing while it works. */
export function WorkCard() {
  const items = createMemo(() => (conv()?.items ?? []).filter((i) => i.role === "tool").slice(-6));
  const live = () => conv()?.state === "working";
  const orch = () => agents().find((a) => a.role === "orchestrator");
  return (
    <Card class="min-h-0 gap-0 rounded-lg py-0 shadow-none">
      <header class="flex items-center gap-2.5 border-b px-4 py-2.5">
        <AgentIcon role="orchestrator" state={live() ? "working" : orch() ? "idle" : undefined} size="sm" />
        <h2 class="text-sm font-semibold">Orchestrator</h2>
        <p class="min-w-0 flex-1 truncate text-sm text-muted-foreground">what it did last</p>
        <span class="num text-xs text-muted-foreground">{live() ? "live" : "quiet"}</span>
      </header>
      <ol class="flex min-h-0 flex-1 flex-col gap-1 px-4 py-3">
        <For each={items()} fallback={<li class="text-sm text-muted-foreground">Its tool calls show here once it starts.</li>}>
          {(it, i) => {
            const active = () => live() && i() === items().length - 1;
            return (
              <li class="m-rise flex h-7 items-center gap-2.5 text-sm">
                <span class="grid size-4 place-items-center">
                  <Show when={active()} fallback={<svg viewBox="0 0 16 16" class="size-4"><path d="M3.5 8.5l3 3 6-7" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></svg>}>
                    <span class="m-breathe size-1.5 rounded-full bg-foreground" />
                  </Show>
                </span>
                <Icon icon={toolIcon(it.tool)} size={15} class="text-muted-foreground" />
                <span class={cx("min-w-0 truncate", active() ? "m-caret" : "text-muted-foreground")}><span class="font-mono text-[13px]">{it.tool}</span> {it.text}</span>
              </li>
            );
          }}
        </For>
      </ol>
    </Card>
  );
}

/** Commits the review has seen, newest round first. */
export function CommitsCard() {
  const reviews = () => runView()?.reviews ?? [];
  const commits = createMemo(() => reviews().flatMap((r) => r.commits).reverse().slice(0, 5));
  const last = () => reviews()[reviews().length - 1];
  return (
    <Card class="gap-0 rounded-lg py-0 shadow-none">
      <header class="flex items-center gap-2.5 border-b px-4 py-2.5">
        <Icon icon={ic.commit} size={16} class="text-muted-foreground" />
        <h2 class="text-sm font-semibold">Commits</h2>
        <p class="num flex-1 text-sm text-muted-foreground">{reviews().length ? `${reviews().length} review round${reviews().length === 1 ? "" : "s"}` : "none yet"}</p>
        <Show when={last()}><Badge variant="outline" class="m-pop font-normal">{last()!.verdict}</Badge></Show>
      </header>
      <ul class="flex flex-col">
        <For each={commits()} fallback={<li class="px-4 py-4 text-sm text-muted-foreground">Commits appear here after the first review. Nothing is committed without your agreement.</li>}>
          {(c) => (
            <li class="m-rise flex items-center gap-3 border-b px-4 py-2.5 last:border-b-0">
              <Icon icon={ic.commit} size={16} class="shrink-0 text-muted-foreground" />
              <span class="min-w-0 flex-1 truncate text-sm">{c.message}</span>
              <span class="font-mono text-xs text-muted-foreground">{c.lot}</span>
              <span class="num w-16 text-right text-xs text-muted-foreground">{c.files.length} file{c.files.length === 1 ? "" : "s"}</span>
            </li>
          )}
        </For>
      </ul>
    </Card>
  );
}

export function ActivityCard() {
  const rows = () => [...crew.log].reverse().slice(0, 8);
  return (
    <Card class="min-h-[160px] gap-0 overflow-hidden rounded-xl py-0 shadow-none">
      <h2 class="border-b px-4 py-2.5 text-sm font-semibold">Activity</h2>
      <ol class="flex flex-col overflow-y-auto px-4 py-1.5">
        <For each={rows()} fallback={<li class="py-3 text-sm text-muted-foreground">Empty until a run starts.</li>}>
          {(e) => <li class="m-rise flex items-baseline gap-3 py-1 text-[13px] leading-snug"><span class="num w-10 shrink-0 font-mono text-xs text-muted-foreground">{clock(e.at)}</span><span><span class="text-foreground">{e.phase}</span> <span class="text-muted-foreground">{e.message}</span></span></li>}
        </For>
      </ol>
    </Card>
  );
}

/** The end of a run. The summary is written out once, the figures follow, the actions rise last. */
export function GateCard() {
  const g = () => crew.gate;
  const [decided, setDecided] = createSignal(false);
  const tw = createTypewriter(g()?.summary_for_user ?? "", 70);
  createEffect(() => {
    if (g()) tw.restart(300);
  });
  const approve = async () => {
    const run = runView()?.run;
    if (!run) return setView("run");
    try {
      await api.runDecide(run, "gate", "gate", "approve", "");
      setDecided(true);
      refetchRun();
      toast({ title: "Decision recorded", body: "Gate approved.", tone: "ok" });
    } catch (e) {
      toast({ title: "Decision failed", body: String(e), tone: "danger" });
    }
  };
  return (
    <Show when={crew.phase === "done" && g()}>
      <Card class="m-rise gap-0 rounded-xl py-0 shadow-none">
        <header class="flex items-center gap-2.5 px-4 py-3">
          <AgentIcon role="orchestrator" state={tw.done() ? "done" : "working"} />
          <div class="flex-1 leading-tight"><p class="text-sm font-medium">{tw.done() ? "Ready for you" : "Wrapping up"}</p><p class="text-xs text-muted-foreground">The teacher's bar is met</p></div>
          <Show when={!tw.done()}><span class="m-breathe text-xs text-muted-foreground">writing</span></Show>
        </header>
        <p class={cx("min-h-[44px] px-4 pb-3 text-sm leading-relaxed", !tw.done() && "m-caret")}>{tw.text()}</p>
        <Show when={tw.done()}>
          <footer class="m-rise flex items-center gap-2 border-t px-4 py-3">
            <Switch>
              <Match when={decided()}><span class="inline-flex items-center gap-2 text-sm text-muted-foreground"><Icon icon={ic.checkBold} size={14} />Approved</span></Match>
              <Match when={true}><Button size="sm" onClick={approve}>Approve the gate</Button><Button size="sm" variant="outline" onClick={() => setView("run")}>Open the run record</Button></Match>
            </Switch>
            <span class="ml-auto text-xs text-muted-foreground">Nothing is committed without you</span>
          </footer>
        </Show>
      </Card>
    </Show>
  );
}

