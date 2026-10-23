// Direction C, Frame (style: framed sheet). One tinted L-shaped frame holds the
// labelled sidebar and a 48px top bar; the work sits on a white sheet with soft
// corners and a hairline edge. The conversation stays the centre of the product:
// decisions arrive as cards in the stream. Its question: "What did the crew ask me?"

import { For, Show } from "solid-js";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Kbd } from "@/components/ui/kbd";
import { Progress, ProgressGroup, ProgressLabel, ProgressValueLabel } from "@/components/ui/progress";
import { Tabs, TabsIndicator, TabsList, TabsTrigger } from "@/components/ui/tabs";
import Icon from "../../../ui/Icon";
import { agents, minutes, needCount, project, projects, stateLabel, usage, views, waited } from "../../data";
import { ic } from "../../icons";
import { AskCard, PermissionRow } from "../../ui/Decision";
import AgentIcon from "../../ui/AgentIcon";
import Composer from "../../ui/Composer";
import { Initials, Mark, StateDot, money } from "../../ui/parts";
import SideRow from "../../ui/SideRow";

export default function Frame() {
  return (
    <div data-dir="frame" class="flex h-full w-full flex-col overflow-hidden bg-frame">
      <header class="flex h-12 shrink-0 items-center gap-3 px-3">
        <div class="flex w-[212px] items-center gap-2 px-1.5 font-semibold"><Mark class="size-5" />harnes</div>
        <Button variant="outline" class="mx-auto h-8 w-[548px] justify-start gap-2 bg-background/60 font-normal text-muted-foreground shadow-none">
          <Icon icon={ic.search} size={15} />Search agents, lots, threads
          <Kbd class="ml-auto">⌘K</Kbd>
        </Button>
        <Button variant="ghost" size="icon" aria-label="Notifications" class="relative size-8 text-muted-foreground">
          <Icon icon={ic.bell} size={17} /><span class="absolute top-1.5 right-1.5 size-1.5 rounded-full bg-signal" />
        </Button>
        <Initials name="Sha Riyat" class="size-7" />
      </header>

      <div class="flex min-h-0 flex-1">
        <aside class="flex w-[228px] shrink-0 flex-col gap-5 px-3 pt-1 pb-3" aria-label="Main">
          <div class="flex flex-col gap-0.5">
            <p class="px-2.5 pb-1 text-xs text-muted-foreground">Projects</p>
            <For each={projects}>{(p) => <SideRow icon={ic.folder} label={p.name} on={p.active} count={p.need} />}</For>
          </div>
          <div class="flex flex-col gap-0.5">
            <p class="px-2.5 pb-1 text-xs text-muted-foreground">Agents · harness-test</p>
            <For each={agents.filter((a) => a.state !== "done")}>
              {(a) => (
                <button class="flex h-8 w-full items-center gap-2.5 rounded-lg px-2.5 text-sm text-muted-foreground hover:bg-accent/60 hover:text-foreground">
                  <StateDot state={a.state} /><span class="truncate">{a.name}</span>
                  <Show when={a.state === "blocked"}><span class="ml-auto text-xs text-foreground">Needs you</span></Show>
                </button>
              )}
            </For>
          </div>
          <div class="mt-auto flex flex-col gap-0.5">
            <SideRow icon={ic.flask} label="Methods" />
            <SideRow icon={ic.gear} label="Settings" />
          </div>
        </aside>

        <section class="mr-2 mb-2 flex min-w-0 flex-1 flex-col overflow-hidden rounded-2xl border bg-background">
          <div class="px-6 pt-5">
            <p class="text-xs text-muted-foreground">{project.scope} / {project.name}</p>
            <h1 class="text-[22px] leading-8 font-semibold tracking-tight">{project.task}</h1>
            <Tabs defaultValue="Chat" class="mt-3 gap-0">
              <TabsList class="h-auto w-full justify-start gap-5 rounded-none border-b bg-transparent p-0 ring-0">
                <For each={views}>
                  {(v) => (
                    <TabsTrigger value={v} class="flex-none px-0 pt-1 pb-2.5 text-muted-foreground data-[selected]:text-foreground dark:data-[selected]:text-foreground">{v}</TabsTrigger>
                  )}
                </For>
                <TabsIndicator class="inset-x-auto top-auto bottom-[-1px] h-0.5 rounded-none border-0 bg-foreground shadow-none dark:bg-foreground" />
              </TabsList>
            </Tabs>
          </div>

          <div class="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_300px]">
            <div class="flex min-h-0 flex-col">
              <div class="flex min-h-0 flex-1 flex-col gap-4 overflow-hidden px-6 py-5">
                <p class="max-w-[560px] self-end rounded-2xl bg-muted px-4 py-2.5 text-sm">{project.task}.</p>
                <p class="max-w-[620px] text-sm leading-relaxed">
                  Crew started. The planner split the work in 6 lots over 2 waves. Worker L1 has been waiting {waited} min on two decisions of yours.
                </p>
                <div class="flex gap-1.5"><Badge variant="outline" class="font-mono text-[11px] font-normal">crew_start</Badge><Badge variant="outline" class="font-mono text-[11px] font-normal">thread_read</Badge></div>
                <Card class="max-w-[620px] gap-3 rounded-xl px-4 py-3.5 shadow-none">
                  <p class="flex items-center gap-2 text-xs text-muted-foreground"><span class="size-1.5 rounded-full bg-signal" />Question · Worker L1</p>
                  <AskCard inline />
                </Card>
                <Card class="max-w-[620px] gap-3 rounded-xl px-4 py-3.5 shadow-none">
                  <p class="flex items-center gap-2 text-xs text-muted-foreground"><span class="size-1.5 rounded-full bg-signal" />Permission · Worker L1</p>
                  <PermissionRow />
                </Card>
              </div>
              <div class="px-6 pb-5">
                <Composer />
              </div>
            </div>

            <aside class="flex flex-col gap-6 border-l px-5 py-5">
              <div class="flex flex-col gap-3">
                <h2 class="text-sm font-semibold">Crew <span class="font-normal text-muted-foreground">· {agents.length}</span></h2>
                <For each={agents}>
                  {(a) => (
                    <p class="flex items-center gap-2.5 text-sm">
                      <AgentIcon role={a.role} state={a.state} size="sm" /><span class="min-w-0 flex-1 truncate">{a.name}</span>
                      <span class="flex items-center gap-1.5 text-xs text-muted-foreground"><StateDot state={a.state} />{stateLabel[a.state]}</span>
                    </p>
                  )}
                </For>
              </div>
              <div class="flex flex-col gap-4">
                <h2 class="text-sm font-semibold">Usage</h2>
                <Progress value={usage.fiveHour * 100} class="gap-1.5">
                  <ProgressGroup class="text-xs"><ProgressLabel class="text-xs font-normal text-muted-foreground">5-hour window · resets in {minutes(usage.resetsIn)}</ProgressLabel><ProgressValueLabel class="num text-xs" /></ProgressGroup>
                </Progress>
                <Progress value={usage.week * 100} class="gap-1.5">
                  <ProgressGroup class="text-xs"><ProgressLabel class="text-xs font-normal text-muted-foreground">Week · resets in {usage.weekResetsInDays} d</ProgressLabel><ProgressValueLabel class="num text-xs" /></ProgressGroup>
                </Progress>
                <p class="num text-xs text-muted-foreground">{money(usage.saved)} tokens saved · {usage.requests} requests</p>
              </div>
              <Show when={needCount}><p class="mt-auto text-xs text-muted-foreground">{needCount} items wait for you in the stream.</p></Show>
            </aside>
          </div>
        </section>
      </div>
    </div>
  );
}
