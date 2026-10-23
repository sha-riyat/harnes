// Direction B, Terminal (style: data terminal). Dark first, dense, long sessions.
// Cards darker than the ground, tabular mono figures, every selection a 1px line
// as wide as its label (ink here: the one hue stays reserved for "needs you").
// Its question: "What is the crew doing, and what is it waiting for?"

import { For, Show, createSignal } from "solid-js";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { cx } from "@/lib/cva";
import Icon from "../../../ui/Icon";
import { agents, events, lotLabel, models, lots, minutes, needCount, NOW, permission, project, question, stateLabel, time, usage, views, waited } from "../../data";
import { ic } from "../../icons";
import { Mark, StateDot, WaveStrip, money } from "../../ui/parts";
import UsageChart from "../../ui/UsageChart";

const rail = [ic.grid, ic.bell, ic.tree, ic.robot, ic.flask];

function Section(props: { title: string; note?: string; class?: string; children: any }) {
  return (
    <section class={cx("flex flex-col gap-2", props.class)}>
      <header class="flex items-baseline gap-2">
        <h2 class="text-xl font-normal text-muted-foreground">{props.title}</h2>
        <Show when={props.note}><span class="num text-xs text-muted-foreground">{props.note}</span></Show>
      </header>
      {props.children}
    </section>
  );
}

export default function Terminal() {
  const [tab, setTab] = createSignal<(typeof views)[number]>("Chat");
  const [pick, setPick] = createSignal("smtp");
  return (
    <div data-dir="terminal" class="flex h-full w-full overflow-hidden">
      <nav class="flex w-[50px] shrink-0 flex-col items-center gap-0.5 border-r py-3" aria-label="Main">
        <Mark class="mb-4 size-5" />
        <For each={rail}>
          {(r, i) => (
            <button aria-label="Section" class={cx("relative grid size-9 place-items-center text-muted-foreground hover:text-foreground", i() === 0 && "text-foreground")}>
              <Show when={i() === 0}><span class="absolute inset-y-2 left-0 w-px bg-foreground" /></Show>
              <Icon icon={r} size={17} />
              <Show when={i() === 1}><span class="absolute top-2 right-2 size-1.5 rounded-full bg-signal" /></Show>
            </button>
          )}
        </For>
        <button aria-label="Settings" class="mt-auto grid size-9 place-items-center text-muted-foreground hover:text-foreground"><Icon icon={ic.gear} size={17} /></button>
      </nav>

      <main class="flex min-w-0 flex-1 flex-col gap-5 px-8 pt-6 pb-5">
        <header class="flex items-end gap-6">
          <div class="flex-1">
            <p class="text-xs text-muted-foreground">{project.scope} · {project.repo}</p>
            <h1 class="text-[26px] leading-9 font-semibold tracking-tight">{project.name}</h1>
          </div>
          <div class="flex gap-5 text-[13px]" role="tablist">
            <For each={views}>
              {(v) => (
                <button role="tab" aria-selected={tab() === v} onClick={() => setTab(v)} class={cx("relative pb-1.5 text-muted-foreground hover:text-foreground", tab() === v && "text-foreground")}>
                  {v}
                  <Show when={tab() === v}><span class="absolute inset-x-0 bottom-0 h-px bg-foreground" /></Show>
                </button>
              )}
            </For>
          </div>
          <p class="num pb-1.5 text-xs text-muted-foreground">updated {time(NOW)}</p>
        </header>

        <div class="grid min-h-0 flex-1 grid-cols-[340px_minmax(0,1fr)_340px] gap-6">
          <div class="flex min-h-0 flex-col gap-5">
            <Section title="Crew" note={`${agents.length} agents`}>
              <Card class="gap-0 rounded-md py-1 shadow-none">
                <For each={agents}>
                  {(a) => (
                    <div class="flex h-9 items-center gap-2.5 px-3.5 text-[13px]">
                      <StateDot state={a.state} />
                      <span class="font-medium">{a.name}</span>
                      <span class="min-w-0 flex-1 truncate text-muted-foreground">{stateLabel[a.state]}</span>
                      <span class="num font-mono text-xs text-muted-foreground">{a.since}</span>
                    </div>
                  )}
                </For>
              </Card>
            </Section>
            <Section title="Lots" note={`${lots.filter((l) => l.status === "done").length} of ${lots.length} done`}>
              <Card class="gap-3 rounded-md px-3.5 py-3 shadow-none">
                <WaveStrip />
                <For each={lots}>
                  {(l) => (
                    <p class="flex items-center gap-2 text-[13px]">
                      <span class="num font-mono text-xs text-muted-foreground">{l.id}</span>
                      <span class="min-w-0 flex-1 truncate">{l.title}</span>
                      <span class={cx("text-xs text-muted-foreground", l.status === "needs" && "text-signal")}>{lotLabel[l.status]}</span>
                    </p>
                  )}
                </For>
              </Card>
            </Section>
          </div>

          <div class="flex min-h-0 flex-col gap-5">
            <Section title="Needs you" note={`${needCount} items · waiting ${waited} min`}>
              <Card class="gap-0 rounded-md py-0 shadow-none">
                <div class="flex flex-col gap-3 p-4">
                  <p class="flex items-center gap-2 text-[13px] text-muted-foreground"><span class="size-1.5 rounded-full bg-signal" />{question.agent} · question · {question.since}</p>
                  <p class="text-base leading-snug font-medium">{question.text}</p>
                  <div class="flex flex-wrap items-center gap-2">
                    <For each={question.options}>
                      {(o) => (
                        <Button variant="outline" size="sm" aria-pressed={pick() === o.id} onClick={() => setPick(o.id)} class={cx("h-8 rounded-full px-3.5", pick() === o.id && "border-foreground bg-accent")}>
                          {o.label}<span class="font-normal text-muted-foreground">{o.detail}</span>
                        </Button>
                      )}
                    </For>
                    <Button size="sm" class="ml-auto h-8 rounded-full px-4">Send</Button>
                    <Button size="sm" variant="ghost" class="h-8 rounded-full text-muted-foreground">Skip</Button>
                  </div>
                </div>
                <div class="flex flex-col gap-3 border-t p-4">
                  <p class="flex items-center gap-2 text-[13px] text-muted-foreground"><span class="size-1.5 rounded-full bg-signal" />{permission.agent} · permission · {permission.since}</p>
                  <code class="rounded bg-muted px-3 py-2 font-mono text-[13px]">{permission.tool} · {permission.command}</code>
                  <div class="flex items-center gap-2">
                    <Button size="sm" class="h-8 rounded-full px-4">Allow once</Button>
                    <Button size="sm" variant="outline" class="h-8 rounded-full px-4">Always</Button>
                    <Button size="sm" variant="ghost" class="h-8 rounded-full text-muted-foreground">Deny</Button>
                  </div>
                </div>
              </Card>
            </Section>
            <Section title="Activity" note="today">
              <Card class="gap-0 rounded-md py-1 shadow-none">
                <For each={events}>
                  {(e) => (
                    <p class="flex h-7 items-center gap-3 px-4 text-[13px]">
                      <span class="num font-mono text-xs text-muted-foreground">{time(e.at)}</span>
                      <span class={cx("min-w-0 truncate", e.ask ? "text-foreground" : "text-muted-foreground")}><span class="text-foreground">{e.who}</span> {e.text}</span>
                    </p>
                  )}
                </For>
              </Card>
            </Section>
          </div>

          <div class="flex min-h-0 flex-col gap-5">
            <Section title="Usage" note="5-hour window">
              <Card class="gap-1 rounded-md px-4 py-3 shadow-none">
                <p class="num flex items-baseline gap-2 font-mono text-xl">{Math.round(usage.fiveHour * 100)}<span class="font-sans text-xs text-muted-foreground">% · resets in {minutes(usage.resetsIn)}</span></p>
                <UsageChart w={310} h={170} />
              </Card>
            </Section>
            <Section title="Totals">
              <Card class="gap-0 divide-y rounded-md py-0 shadow-none">
                <p class="flex h-10 items-center justify-between px-4 text-[13px]"><span class="text-muted-foreground">Week</span><span class="num font-mono">{Math.round(usage.week * 100)}% · resets in {usage.weekResetsInDays} d</span></p>
                <p class="flex h-10 items-center justify-between px-4 text-[13px]"><span class="text-muted-foreground">Tokens saved</span><span class="num font-mono">{money(usage.saved)}</span></p>
                <p class="flex h-10 items-center justify-between px-4 text-[13px]"><span class="text-muted-foreground">Requests compressed</span><span class="num font-mono">{usage.requests}</span></p>
              </Card>
            </Section>
            <Section title="Models" note="agents on each">
              <Card class="gap-0 divide-y rounded-md py-0 shadow-none">
                <For each={models}>{(m) => <p class="flex h-10 items-center justify-between px-4 text-[13px]"><span>{m.model}</span><span class="num font-mono text-muted-foreground">{m.agents}</span></p>}</For>
              </Card>
            </Section>
          </div>
        </div>
      </main>
    </div>
  );
}
