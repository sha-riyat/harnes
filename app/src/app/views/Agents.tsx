// The crew as icons, and every agent session found on the machine. A tile shows the
// role's icon (a ring while it works, the one hue while it waits for you), its name,
// its provider and its state; a click opens its terminal. Below, what the lookout
// sees on this machine, with the changes of the last hour.

import { For, Show, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import { api, on, type LookSession, type Lookout } from "../../api";
import { Badge } from "@/components/ui/badge";
import { Card } from "@/components/ui/card";
import { cx } from "@/lib/cva";
import { agents } from "../../model";
import { panes, setDrawerPane, setFocused, setView } from "../../store";
import Icon from "../../ui/Icon";
import AgentIcon from "../../ui/kit/AgentIcon";
import { roleHint, visualLabel } from "../../ui/kit/roles";
import { ic } from "../../ui/ph";
import ThreadsCard from "../cards/Threads";
import { ago as relative } from "../Panels";

const LABEL: Record<string, string> = { "needs-you": "Needs you", working: "Working", idle: "Idle", finished: "Done", failed: "Failed", unknown: "Unknown" };
const dot = (s: string) => (s === "needs-you" ? "bg-signal ring-[3px] ring-signal/25" : s === "working" ? "bg-foreground" : s === "failed" ? "bg-destructive" : "border border-muted-foreground");

export default function Agents() {
  const [data, setData] = createSignal<Lookout | null>(null);
  const refresh = () => api.lookout().then(setData).catch(() => {});
  onMount(async () => {
    refresh();
    const off = await on("lookout-changed", refresh);
    const t = setInterval(refresh, 5000);
    onCleanup(() => (off(), clearInterval(t)));
  });
  const sessions = () => data()?.snapshot.sessions ?? [];
  const paneOf = (s: LookSession) => panes.find((p) => (s.pid && p.pid === s.pid) || (s.harnes_key && p.key === s.harnes_key));
  const jump = (s: LookSession) => {
    const p = paneOf(s);
    if (p) (setFocused(p.id), setView("panes"));
  };
  const needs = createMemo(() => sessions().filter((s) => s.status === "needs-you").length);

  return (
    <div class="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_340px] min-[1100px]:grid-cols-[minmax(0,1fr)_420px] gap-4 overflow-hidden p-5">
      <div class="flex min-h-0 flex-col gap-4 overflow-y-auto">
        <Card class="gap-0 rounded-xl py-0 shadow-none">
          <header class="flex items-center border-b px-4 py-3"><p class="text-sm font-medium">Crew <span class="font-normal text-muted-foreground">· {agents().length} agent{agents().length === 1 ? "" : "s"}</span></p></header>
          <ul class="grid grid-cols-2 gap-px bg-border [&>li:last-child:nth-child(odd)]:col-span-2">
            <For each={agents()} fallback={<li class="col-span-2 bg-card px-4 py-6 text-sm text-muted-foreground">No agent yet. They start when you give the orchestrator a task.</li>}>
              {(a, i) => (
                <li class="bg-card" style={{ "--i": i() }}>
                  <button class="m-rise flex w-full gap-3 px-4 py-3.5 text-left transition-colors duration-150 hover:bg-accent/40" style={{ "--i": i() }} onClick={() => setDrawerPane(a.paneId)} title="Open its terminal">
                    <AgentIcon role={a.role} state={a.state} size="lg" />
                    <div class="min-w-0 flex-1 leading-tight">
                      <p class="flex items-center gap-2 text-sm font-medium">{a.name}<span class="text-xs font-normal text-muted-foreground">{a.provider}</span></p>
                      <p class="text-xs text-muted-foreground">{roleHint[a.role]}</p>
                      <p class={cx("mt-1.5 text-xs", a.state === "blocked" ? "text-foreground" : "text-muted-foreground")}>{visualLabel[a.state]}<Show when={a.lot}> · lot {a.lot}</Show></p>
                    </div>
                  </button>
                </li>
              )}
            </For>
          </ul>
        </Card>

        <Card class="gap-0 overflow-hidden rounded-xl py-0 shadow-none">
          <header class="flex items-center gap-2 border-b px-4 py-3"><Icon icon={ic.machine} size={16} class="text-muted-foreground" /><p class="text-sm font-medium">On this machine <span class="font-normal text-muted-foreground">· {sessions().length} session{sessions().length === 1 ? "" : "s"}</span></p><Show when={needs()}><Badge variant="secondary" class="num ml-1">{needs()} need you</Badge></Show></header>
          <table class="w-full text-sm">
            <thead><tr class="text-left text-xs text-muted-foreground"><th class="w-8 px-4 py-2 font-normal" /><th class="py-2 font-normal">Agent</th><th class="py-2 font-normal">Provider</th><th class="py-2 font-normal">Project</th><th class="py-2 font-normal">Status</th><th class="py-2 font-normal">Since</th><th class="px-4 py-2 text-right font-normal">Tokens</th></tr></thead>
            <tbody>
              <For each={sessions()} fallback={<tr><td colSpan={7} class="px-4 py-6 text-muted-foreground">No session detected.</td></tr>}>
                {(s) => (
                  <tr class={cx("m-rise border-t transition-colors", paneOf(s) && "cursor-pointer hover:bg-accent/40")} onClick={() => jump(s)}>
                    <td class="px-4 py-2"><span class={cx("block size-2 rounded-full", dot(s.status))} /></td>
                    <td class="py-2 font-medium">{s.name}<Show when={paneOf(s)}><Badge variant="outline" class="ml-2 font-normal">harnes</Badge></Show></td>
                    <td class="py-2 text-muted-foreground">{s.provider}</td>
                    <td class="py-2 text-muted-foreground" title={s.cwd}>{s.project}</td>
                    <td class="py-2">{LABEL[s.status]}</td>
                    <td class="num py-2 text-muted-foreground">{relative(s.status_since)}</td>
                    <td class="num px-4 py-2 text-right text-muted-foreground">{s.tokens ? `${s.tokens.input + s.tokens.cached} / ${s.tokens.output}` : "–"}</td>
                  </tr>
                )}
              </For>
            </tbody>
          </table>
          <Show when={(data()?.events ?? []).length}>
            <ol class="flex flex-col gap-1 border-t px-4 py-3 text-[13px]">
              <For each={data()!.events.slice(0, 6)}>{(e) => <li class="flex gap-3 text-muted-foreground"><span class="num w-12 shrink-0 text-xs">{new Date(e.at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</span><span><span class="text-foreground">{e.session_name}</span> {e.kind === "appeared" ? "appeared" : e.kind === "ended" ? "ended" : `${LABEL[e.from ?? "unknown"]} → ${LABEL[e.to ?? "unknown"]}`}</span></li>}</For>
            </ol>
          </Show>
        </Card>
      </div>
      <ThreadsCard class="h-full" />
    </div>
  );
}
