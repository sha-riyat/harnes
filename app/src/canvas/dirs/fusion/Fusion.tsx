// Direction D, Fusion of A Ledger and C Frame. The Frame's shell (tinted L-frame,
// labelled sidebar, 48px top bar, a white sheet) around the Ledger's overview:
// the one question first, the run as a strip, one divided row of figures. What the
// crew does is alive (work log, thread, pull requests), and what waits for the user
// is a pair of cards: answering them folds them and sends Worker L1 back to work,
// in the sidebar, the strip and the figures at once. Tabs: Overview, Chat, Run, Agents.

import { For, Match, Show, Switch, createSignal } from "solid-js";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Kbd } from "@/components/ui/kbd";
import { Progress, ProgressGroup, ProgressLabel, ProgressValueLabel } from "@/components/ui/progress";
import { Tabs, TabsIndicator, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cx } from "@/lib/cva";
import Icon from "../../../ui/Icon";
import { agents, events, lots, lotsDone, minutes, NOW, project, projects, run, runElapsed, time, usage, waited, type LotStatus } from "../../data";
import { ic } from "../../icons";
import AgentsCard from "../../cards/AgentsCard";
import DiffCard from "../../cards/DiffCard";
import MergeCard from "../../cards/MergeCard";
import PermissionCard from "../../cards/PermissionCard";
import PlanCard from "../../cards/PlanCard";
import QuestionCard from "../../cards/QuestionCard";
import TestsCard from "../../cards/TestsCard";
import AgentIcon from "../../ui/AgentIcon";
import Composer from "../../ui/Composer";
import Pop from "../../ui/Pop";
import MethodsView from "../../views/MethodsView";
import { NotificationsPanel, ProfileMenu } from "../../views/Panels";
import SettingsView from "../../views/SettingsView";
import { setTheme, theme } from "../../../ui/state";
import { createFlow, FlowProvider, useFlow } from "../../ui/flow";
import { Initials, Mark, WaveStrip, money } from "../../ui/parts";
import SideRow from "../../ui/SideRow";
import { createSim, merged } from "../../ui/sim";
import { ThinkingCard } from "../generative/Thinking";
import LiveWork from "./LiveWork";
import PullRequests from "./PullRequests";
import Threads from "./Threads";

const tabs = ["Overview", "Chat", "Run", "Agents"] as const;

function Cell(props: { label: string; note?: string; signal?: boolean; children: any }) {
  return (
    <div class="flex flex-col gap-0.5 px-5 py-3">
      <p class="text-xs text-muted-foreground">{props.label}</p>
      <p class={cx("num flex items-baseline gap-1 text-[28px] leading-9 tracking-tight", props.signal && "text-signal")}>{props.children}</p>
      <p class="text-xs text-muted-foreground">{props.note ?? " "}</p>
    </div>
  );
}

function Overview(props: { t: () => number }) {
  const flow = useFlow();
  const t = props.t;
  const done = () => lotsDone + (merged(t()) ? 1 : 0);
  const working = () => agents.filter((a) => flow.stateOf(a) === "working").length;
  const override = (): Record<string, LotStatus> => ({ ...(merged(t()) ? { L5: "done" } : {}), ...(flow.need() === 0 ? { L1: "working" } : {}) });
  return (
    <div class="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_340px] gap-4 p-5">
      <div class="flex min-h-0 flex-col gap-4">
        <Card class="gap-0 rounded-lg py-0 shadow-none">
          <div class="flex flex-col gap-2 px-5 py-4">
            <p class="text-xs text-muted-foreground">Current run</p>
            <h2 class="text-base leading-snug font-semibold">{project.task}</h2>
            <p class="num text-sm text-muted-foreground">Wave {run.wave} of {run.waves} · started {time(run.started)} · {minutes(runElapsed)}</p>
            <WaveStrip class="mt-1" override={override()} />
          </div>
          <div class="grid grid-cols-4 divide-x border-t">
            <Cell label="Working" note="agents"><Show when={working()} keyed>{(n) => <span class="m-tick">{n}</span>}</Show><span class="text-sm text-muted-foreground">/ {agents.length}</span></Cell>
            <Cell label="Needs you" note={flow.need() ? `waiting ${waited} min` : "all answered"} signal={flow.need() > 0}><Show when={flow.waiting()} keyed>{(n) => <span class="m-tick">{n}</span>}</Show></Cell>
            <Cell label="Lots done" note={merged(t()) ? "L5 merged just now" : "wave 1 of 2"}><Show when={done()} keyed>{(n) => <span class="m-tick">{n}</span>}</Show><span class="text-sm text-muted-foreground">/ {lots.length}</span></Cell>
            <Cell label="5-hour usage" note={`resets in ${minutes(usage.resetsIn)}`}><span>{Math.round(usage.fiveHour * 100)}</span><span class="text-sm text-muted-foreground">%</span></Cell>
          </div>
        </Card>
        <div class="grid min-h-0 flex-1 grid-cols-2 gap-4"><LiveWork t={t} /><Threads t={t} /></div>
        <PullRequests t={t} />
      </div>
      <aside class="flex min-h-0 flex-col gap-3">
        <div class="flex items-center justify-between px-1"><h2 class="text-sm font-semibold">Needs you</h2><Badge variant="secondary" class="num">{flow.need()}</Badge></div>
        <QuestionCard />
        <PermissionCard />
        <Card class="min-h-0 flex-1 gap-0 overflow-hidden rounded-xl py-0 shadow-none">
          <h2 class="border-b px-4 py-2.5 text-sm font-semibold">Activity</h2>
          <ol class="flex flex-col px-4 py-1.5">
            <For each={events.slice(0, 3)}>{(e) => <li class="flex items-baseline gap-3 py-1 text-[13px] leading-snug"><span class="num w-10 shrink-0 font-mono text-xs text-muted-foreground">{time(e.at)}</span><span class={cx(!e.ask && "text-muted-foreground")}><span class="text-foreground">{e.who}</span> {e.text}</span></li>}</For>
          </ol>
        </Card>
      </aside>
    </div>
  );
}

function Scroll(props: { children: any; class?: string }) {
  return <div class={cx("min-h-0 flex-1 overflow-y-auto p-5", props.class)}>{props.children}</div>;
}

type View = "project" | "methods" | "settings";
type Panel = "notifications" | "profile" | null;

export default function Fusion(props: { initialView?: View; initialPanel?: Panel; initialSection?: string; initialTab?: (typeof tabs)[number] }) {
  const flow = createFlow();
  const sim = createSim();
  const [tab, setTab] = createSignal<(typeof tabs)[number]>(props.initialTab ?? "Overview");
  const [view, setView] = createSignal<View>(props.initialView ?? "project");
  const [panel, setPanel] = createSignal<Panel>(props.initialPanel ?? null);
  const [section, setSection] = createSignal(props.initialSection);
  const live = () => sim.t() >= 9;
  const toProject = (t: (typeof tabs)[number] = "Overview") => (setView("project"), setTab(t), setPanel(null));
  const toSettings = (id?: string) => (setSection(id), setView("settings"), setPanel(null));
  return (
    <FlowProvider flow={flow}>
      <div data-dir="fusion" class="flex h-full w-full flex-col overflow-hidden bg-frame">
        <header class="flex h-12 shrink-0 items-center gap-3 px-3">
          <div class="flex w-[212px] items-center gap-2 px-1.5 font-semibold"><Mark class="size-5" />harnes</div>
          <Button variant="outline" class="mx-auto h-8 w-[548px] justify-start gap-2 bg-background/60 font-normal text-muted-foreground shadow-none"><Icon icon={ic.search} size={15} />Search agents, lots, threads<Kbd class="ml-auto">⌘K</Kbd></Button>
          <div class="relative">
            <Button variant="ghost" size="icon" aria-label="Notifications" aria-expanded={panel() === "notifications"} class={cx("relative size-8 text-muted-foreground", panel() === "notifications" && "bg-accent text-foreground")} onClick={() => setPanel(panel() === "notifications" ? null : "notifications")}>
              <Icon icon={ic.bell} size={17} /><Show when={flow.need() > 0 || live()}><span class="m-pop absolute top-1.5 right-1.5 size-1.5 rounded-full bg-signal" /></Show>
            </Button>
            <Pop open={panel() === "notifications"} onClose={() => setPanel(null)} align="right" label="Notifications"><NotificationsPanel live={live()} onOpenSettings={() => toSettings("notify")} onAnswer={() => toProject("Overview")} /></Pop>
          </div>
          <div class="relative">
            <button aria-label="Profile" aria-expanded={panel() === "profile"} class="rounded-full ring-offset-2 ring-offset-frame transition-shadow hover:ring-2 hover:ring-foreground/15 aria-expanded:ring-2 aria-expanded:ring-foreground/25" onClick={() => setPanel(panel() === "profile" ? null : "profile")}><Initials name="Sha Riyat" class="size-7" /></button>
            <Pop open={panel() === "profile"} onClose={() => setPanel(null)} align="right" label="Profile"><ProfileMenu theme={theme()} onTheme={setTheme} onSettings={() => toSettings()} /></Pop>
          </div>
        </header>

        <div class="flex min-h-0 flex-1">
          <aside class="flex w-[228px] shrink-0 flex-col gap-5 px-3 pt-1 pb-3" aria-label="Main">
            <div class="flex flex-col gap-0.5">
              <p class="px-2.5 pb-1 text-xs text-muted-foreground">Projects</p>
              <For each={projects}>{(p) => <SideRow icon={ic.folder} label={p.name} on={p.active && view() === "project"} count={p.active ? flow.need() : p.need} onClick={() => toProject()} />}</For>
            </div>
            <div class="flex flex-col gap-0.5">
              <p class="px-2.5 pb-1 text-xs text-muted-foreground">Agents · harness-test</p>
              <For each={agents.filter((a) => a.state !== "done")}>
                {(a) => (
                  <button class="flex h-9 w-full items-center gap-2.5 rounded-lg px-2 text-sm text-muted-foreground transition-colors hover:bg-accent/60 hover:text-foreground">
                    <AgentIcon role={a.role} state={flow.stateOf(a)} size="sm" /><span class="truncate">{a.name}</span>
                    <Show when={flow.stateOf(a) === "blocked"}><span class="m-pop ml-auto text-xs text-foreground">Needs you</span></Show>
                  </button>
                )}
              </For>
            </div>
            <div class="mt-auto flex flex-col gap-3">
              <div class="flex flex-col gap-2 px-2.5">
                <Progress value={usage.week * 100} class="gap-1.5"><ProgressGroup><ProgressLabel class="text-xs font-normal text-muted-foreground">Week · {usage.weekResetsInDays} d</ProgressLabel><ProgressValueLabel class="num text-xs" /></ProgressGroup></Progress>
                <p class="num text-xs text-muted-foreground">{money(usage.saved)} tokens saved</p>
              </div>
              <div class="flex flex-col gap-0.5"><SideRow icon={ic.flask} label="Methods" on={view() === "methods"} onClick={() => (setView("methods"), setPanel(null))} /><SideRow icon={ic.gear} label="Settings" on={view() === "settings"} onClick={() => toSettings()} /></div>
            </div>
          </aside>

          <section class="mr-2 mb-2 flex min-w-0 flex-1 flex-col overflow-hidden rounded-2xl border bg-background">
            <Switch>
              <Match when={view() === "methods"}><MethodsView /></Match>
              <Match when={view() === "settings"}><SettingsView initial={section()} /></Match>
              <Match when={view() === "project"}>
            <div class="px-6 pt-5">
              <div class="flex items-end gap-4">
                <div class="min-w-0 flex-1">
                  <p class="text-xs text-muted-foreground">{project.scope} / Projects</p>
                  <div class="flex items-center gap-3">
                    <h1 class="text-2xl font-semibold tracking-tight">{project.name}</h1>
                    <Show when={flow.need() > 0} fallback={<Badge variant="outline" class="m-pop gap-1.5 font-normal text-muted-foreground"><Icon icon={ic.checkBold} size={11} />All answered</Badge>}>
                      <Badge variant="outline" class="gap-1.5 text-foreground"><span class="m-ring size-1.5 rounded-full bg-signal" />Needs you</Badge>
                    </Show>
                  </div>
                  <p class="mt-0.5 text-sm text-muted-foreground">{project.repo} · <span class="font-mono text-xs">{project.branch}</span> · updated {time(NOW)}</p>
                </div>
                <Button variant="ghost" size="icon" aria-label={sim.paused() ? "Resume the replay" : "Pause the replay"} title="Replay of the crew" class="text-muted-foreground" onClick={sim.toggle}><Icon icon={sim.paused() ? ic.play : ic.pause} size={17} /></Button>
                <Button variant="ghost" size="icon" aria-label="Replay from the start" title="Replay from the start" class="text-muted-foreground" onClick={() => (sim.replay(), flow.reset())}><Icon icon={ic.replay} size={17} /></Button>
                <Button variant="outline">Stop run</Button>
              </div>
              <Tabs value={tab()} onChange={(v) => setTab(v as (typeof tabs)[number])} class="mt-3 gap-0">
                <TabsList class="h-auto w-full justify-start gap-5 rounded-none border-b bg-transparent p-0 ring-0">
                  <For each={tabs}>{(v) => <TabsTrigger value={v} class="flex-none px-0 pt-1 pb-2.5 text-muted-foreground data-[selected]:text-foreground dark:data-[selected]:text-foreground">{v}</TabsTrigger>}</For>
                  <TabsIndicator class="inset-x-auto top-auto bottom-[-1px] h-0.5 rounded-none border-0 bg-foreground shadow-none dark:bg-foreground" />
                </TabsList>
              </Tabs>
            </div>

            <Switch>
              <Match when={tab() === "Overview"}><Overview t={sim.t} /></Match>
              <Match when={tab() === "Chat"}>
                <Scroll class="flex flex-col pb-0">
                  <div class="mx-auto flex w-full max-w-[700px] flex-1 flex-col gap-4">
                    <p class="m-rise max-w-[520px] self-end rounded-2xl bg-muted px-4 py-2.5 text-sm">{project.task}.</p>
                    <div class="m-rise flex gap-3" style={{ "--i": 1 }}><AgentIcon role="orchestrator" state="working" /><p class="max-w-[600px] text-sm leading-relaxed">Crew started. The planner split the work in 6 lots over 2 waves. Worker L1 has a question and needs a command allowed; both are below.</p></div>
                    <PlanCard />
                    <ThinkingCard />
                    <QuestionCard />
                    <PermissionCard />
                    <Composer class="sticky bottom-0 mt-auto bg-background pt-2 pb-5" />
                  </div>
                </Scroll>
              </Match>
              <Match when={tab() === "Run"}>
                <Scroll class="flex flex-col gap-4">
                  <div class="grid grid-cols-[640px_minmax(0,1fr)] gap-4"><PlanCard /><TestsCard /></div>
                  <div class="grid grid-cols-2 gap-4"><MergeCard /><DiffCard /></div>
                </Scroll>
              </Match>
              <Match when={tab() === "Agents"}>
                <Scroll class="grid grid-cols-[minmax(0,1fr)_340px] items-start gap-4"><AgentsCard /><div class="flex flex-col gap-4"><LiveWork t={sim.t} /><Threads t={sim.t} /></div></Scroll>
              </Match>
            </Switch>
              </Match>
            </Switch>
          </section>
        </div>
      </div>
    </FlowProvider>
  );
}
