// Direction A, Ledger (style: achromatic ledger). Light first, an icon rail,
// white cards on a near-white ground. No colour in the chrome: the one hue marks
// what needs you. Its question: "Does the crew need me?" The finding sits in the
// title row and in the lead card: Worker L1 has waited 4 min on two items.

import { For, Show } from "solid-js";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { cx } from "@/lib/cva";
import Icon from "../../../ui/Icon";
import { agents, lots, lotsDone, minutes, needCount, NOW, project, runElapsed, stateLabel, time, usage, waited, run } from "../../data";
import { ic } from "../../icons";
import { AskCard, PermissionRow } from "../../ui/Decision";
import AgentIcon from "../../ui/AgentIcon";
import { Mark, StateDot, WaveStrip, money } from "../../ui/parts";
import UsageChart from "../../ui/UsageChart";

const rail = [
  { icon: ic.grid, label: "Overview", on: true },
  { icon: ic.bell, label: "Inbox", badge: true },
  { icon: ic.tree, label: "Run" },
  { icon: ic.robot, label: "Agents" },
  { icon: ic.flask, label: "Methods" },
];

function Cell(props: { label: string; value: string; unit?: string; note?: string; signal?: boolean }) {
  return (
    <div class="flex flex-col gap-0.5 px-5 py-3">
      <p class="text-xs text-muted-foreground">{props.label}</p>
      <p class="num flex items-baseline gap-1 text-[30px] leading-9 tracking-tight">
        <span class={cx(props.signal && "text-signal")}>{props.value}</span>
        <Show when={props.unit}><span class="text-sm text-muted-foreground">{props.unit}</span></Show>
      </p>
      <p class="text-xs text-muted-foreground">{props.note ?? " "}</p>
    </div>
  );
}

export default function Ledger() {
  const working = agents.filter((a) => a.state === "working").length;
  return (
    <div data-dir="ledger" class="flex h-full w-full overflow-hidden">
      <nav class="flex w-14 shrink-0 flex-col items-center gap-1 border-r py-3" aria-label="Main">
        <Mark class="mb-3 size-6" />
        <For each={rail}>
          {(r) => (
            <Button variant="ghost" size="icon" aria-label={r.label} title={r.label} class={cx("relative text-muted-foreground", r.on && "bg-accent text-foreground")}>
              <Icon icon={r.icon} size={18} />
              <Show when={r.badge}><span class="absolute top-1.5 right-1.5 size-2 rounded-full bg-signal" /></Show>
            </Button>
          )}
        </For>
        <Button variant="ghost" size="icon" aria-label="Settings" title="Settings" class="mt-auto text-muted-foreground"><Icon icon={ic.gear} size={18} /></Button>
      </nav>

      <main class="flex min-w-0 flex-1 flex-col gap-3 p-5">
        <header class="flex items-end gap-4">
          <div class="min-w-0 flex-1">
            <p class="text-xs text-muted-foreground">{project.scope} / Projects</p>
            <div class="flex items-center gap-3">
              <h1 class="text-2xl font-semibold tracking-tight">{project.name}</h1>
              <Badge variant="outline" class="gap-1.5 text-foreground"><span class="size-1.5 rounded-full bg-signal" />Needs you</Badge>
            </div>
            <p class="mt-0.5 text-sm text-muted-foreground">
              {project.repo} · <span class="font-mono text-xs">{project.branch}</span> · updated {time(NOW)}
            </p>
          </div>
          <Button variant="outline">Stop run</Button>
          <Button>Answer Worker L1</Button>
        </header>

        <div class="grid grid-cols-3 gap-3">
          <div class="col-span-2 flex flex-col gap-3">
            <Card class="gap-0 rounded-lg py-0 shadow-none">
              <div class="flex flex-col gap-2 px-5 py-4">
                <p class="text-xs text-muted-foreground">Current run</p>
                <h2 class="text-base leading-snug font-semibold">{project.task}</h2>
                <p class="num text-sm text-muted-foreground">Wave {run.wave} of {run.waves} · started {time(run.started)} · {minutes(runElapsed)}</p>
                <WaveStrip class="mt-1" />
              </div>
              <div class="grid grid-cols-4 divide-x border-t">
                <Cell label="Working" value={String(working)} unit={`/ ${agents.length}`} note="agents" />
                <Cell label="Needs you" value="1" note={`waiting ${waited} min`} signal />
                <Cell label="Lots done" value={String(lotsDone)} unit={`/ ${lots.length}`} note="wave 1 of 2" />
                <Cell label="Run time" value={String(runElapsed)} unit="min" note={`since ${time(run.started)}`} />
              </div>
            </Card>
            <Card class="flex-1 gap-1 rounded-lg px-5 py-4 shadow-none">
              <div class="flex items-end justify-between">
                <div>
                  <p class="text-xs text-muted-foreground">Usage, 5-hour window</p>
                  <p class="num flex items-baseline gap-2 text-[30px] leading-9 tracking-tight">{Math.round(usage.fiveHour * 100)}<span class="text-sm text-muted-foreground">% · resets in {minutes(usage.resetsIn)}</span></p>
                </div>
                <p class="num text-right text-xs text-muted-foreground">
                  Week {Math.round(usage.week * 100)}% · resets in {usage.weekResetsInDays} d<br />
                  {money(usage.saved)} tokens saved · {usage.requests} requests
                </p>
              </div>
              <UsageChart w={900} h={128} />
            </Card>
          </div>
          <Card class="gap-4 rounded-lg px-5 py-4 shadow-none">
            <div class="flex items-center justify-between">
              <h2 class="text-base font-semibold">Needs you</h2>
              <Badge variant="secondary" class="num">{needCount}</Badge>
            </div>
            <AskCard inline />
            <Separator />
            <PermissionRow />
          </Card>
        </div>

        <Card class="min-h-0 flex-1 gap-0 overflow-hidden rounded-lg py-0 shadow-none">
          <div class="flex items-baseline gap-2 px-5 pt-3 pb-2">
            <h2 class="text-base font-semibold">Crew</h2>
            <p class="text-sm text-muted-foreground">{agents.length} agents</p>
            <Button variant="link" size="sm" class="ml-auto h-auto p-0 text-muted-foreground">View all</Button>
          </div>
          <Table>
            <TableHeader>
              <TableRow class="hover:bg-transparent">
                <TableHead class="h-8 pl-5 text-xs font-normal text-muted-foreground">Agent</TableHead>
                <TableHead class="h-8 text-xs font-normal text-muted-foreground">Model</TableHead>
                <TableHead class="h-8 text-xs font-normal text-muted-foreground">State</TableHead>
                <TableHead class="h-8 text-xs font-normal text-muted-foreground">Task</TableHead>
                <TableHead class="h-8 text-xs font-normal text-muted-foreground">Lot</TableHead>
                <TableHead class="h-8 text-right text-xs font-normal text-muted-foreground">Since</TableHead>
                <TableHead class="h-8 w-24 pr-5" />
              </TableRow>
            </TableHeader>
            <TableBody>
              <For each={agents}>
                {(a) => (
                  <TableRow class="h-[34px]">
                    <TableCell class="py-1 pl-5"><span class="flex items-center gap-2.5 font-medium"><AgentIcon role={a.role} state={a.state} size="sm" />{a.name}</span></TableCell>
                    <TableCell class="py-1 text-muted-foreground">{a.model}</TableCell>
                    <TableCell class="py-1"><span class="flex items-center gap-2"><StateDot state={a.state} />{stateLabel[a.state]}</span></TableCell>
                    <TableCell class="py-1 text-muted-foreground">{a.task}</TableCell>
                    <TableCell class="num py-1 text-muted-foreground">{a.lot ?? "–"}</TableCell>
                    <TableCell class="num py-1 text-right text-muted-foreground">{a.since}</TableCell>
                    <TableCell class="py-1 pr-5 text-right"><Show when={a.state === "blocked"}><Button variant="outline" size="sm" class="h-7">Answer</Button></Show></TableCell>
                  </TableRow>
                )}
              </For>
            </TableBody>
          </Table>
        </Card>
      </main>
    </div>
  );
}
