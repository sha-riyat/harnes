// The panels under the top bar and the composer: notifications (the bell), the
// profile menu (the avatar) and the usage panel (the ring in the message box).
// Everything is read from what harnes already keeps; nothing is made up.

import { getVersion } from "@tauri-apps/api/app";
import { For, Show, createResource, createSignal } from "solid-js";
import { api } from "../api";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Progress, ProgressGroup, ProgressLabel, ProgressValueLabel } from "@/components/ui/progress";
import { cx } from "@/lib/cva";
import { agents, budget, isRunning, isUnread, left, markNoticesRead, notices, unreadCount, type Notice } from "../model";
import { setView, stats } from "../store";
import Icon from "../ui/Icon";
import AgentIcon from "../ui/kit/AgentIcon";
import { ic } from "../ui/ph";
import { setPaletteOpen, setSettingsSection, setTheme, theme, toast } from "../ui/state";

export const ago = (ms: number) => {
  const s = Math.max(0, Math.round((Date.now() - ms) / 1000));
  if (s < 45) return "just now";
  if (s < 3600) return `${Math.max(1, Math.round(s / 60))} min`;
  if (s < 86400) return `${Math.round(s / 3600)} h`;
  return `${Math.round(s / 86400)} d`;
};

const fmt = (n?: number) => (n === undefined ? "–" : n >= 1e6 ? `${(n / 1e6).toFixed(1)}M` : n >= 1e3 ? `${(n / 1e3).toFixed(1)}k` : `${n}`);

export function NotificationsPanel(props: { onClose: () => void }) {
  const [filter, setFilter] = createSignal<"all" | "ask">("all");
  const shown = () => notices().filter((n) => filter() === "all" || n.ask);
  const go = (n: Notice) => {
    props.onClose();
    setView(n.action === "Review" || n.action === "Answer" ? "overview" : "run");
  };
  return (
    <div class="w-[400px]">
      <header class="flex items-center gap-2 px-4 pt-3.5 pb-2">
        <h2 class="text-sm font-semibold">Notifications</h2>
        <Show when={unreadCount()} keyed>{(n) => <Badge variant="secondary" class="num m-tick">{n}</Badge>}</Show>
        <Button variant="ghost" size="sm" class="ml-auto h-7 text-muted-foreground" disabled={!notices().some((n) => !n.ask && isUnread(n))} onClick={markNoticesRead}>Mark all read</Button>
      </header>
      <div class="flex gap-1 px-3 pb-2" role="tablist">
        <For each={[["all", "All"], ["ask", "Needs you"]] as const}>
          {([id, label]) => <button role="tab" aria-selected={filter() === id} onClick={() => setFilter(id)} class={cx("h-7 rounded-full px-3 text-xs transition-colors", filter() === id ? "bg-accent text-foreground" : "text-muted-foreground hover:text-foreground")}>{label}</button>}
        </For>
      </div>
      <ul class="flex max-h-[360px] flex-col overflow-y-auto border-t px-1.5 py-1.5">
        <For each={shown()} fallback={<li class="m-rise px-4 py-8 text-center text-sm text-muted-foreground">Nothing waits for you.</li>}>
          {(n) => (
            <li class="m-rise flex gap-3 rounded-lg px-2.5 py-2.5 transition-colors hover:bg-accent/50">
              <AgentIcon role={n.role} state={n.ask ? "blocked" : undefined} size="md" />
              <div class="min-w-0 flex-1 text-sm leading-snug">
                <p><span class="font-medium">{n.who}</span> <span class="text-muted-foreground">{n.text}</span></p>
                <p class="num mt-0.5 text-xs text-muted-foreground">{ago(n.at)}</p>
              </div>
              <div class="flex shrink-0 flex-col items-end gap-1.5">
                <Show when={isUnread(n)}><span class={cx("mt-1.5 size-1.5 rounded-full", n.ask ? "bg-signal" : "bg-foreground")} /></Show>
                <Show when={n.action}><Button variant="outline" size="sm" class="h-7" onClick={() => go(n)}>{n.action}</Button></Show>
              </div>
            </li>
          )}
        </For>
      </ul>
      <footer class="flex items-center border-t px-4 py-2.5 text-sm">
        <button class="inline-flex items-center gap-2 text-muted-foreground transition-colors hover:text-foreground" onClick={() => (props.onClose(), setSettingsSection("notices"), setView("settings"))}><Icon icon={ic.gear} size={15} />Notification settings</button>
      </footer>
    </div>
  );
}

function Item(props: { icon: string; label: string; hint?: string; onClick?: () => void }) {
  return (
    <button class="flex h-9 w-full items-center gap-2.5 rounded-lg px-2.5 text-sm transition-colors hover:bg-accent" onClick={props.onClick}>
      <Icon icon={props.icon} size={16} class="text-muted-foreground" />{props.label}
      <span class="ml-auto text-xs text-muted-foreground">{props.hint}</span>
    </button>
  );
}

export function ProfileMenu(props: { onClose: () => void }) {
  const [version] = createResource(() => getVersion().catch(() => ""));
  const l = () => budget()?.limits;
  return (
    <div class="w-[300px]">
      <div class="flex items-center gap-3 px-4 py-3.5">
        <span class="grid size-10 place-items-center rounded-full bg-secondary text-xs font-medium">h</span>
        <div class="min-w-0 flex-1 leading-tight"><p class="text-sm font-semibold">Local session</p><p class="text-xs text-muted-foreground">Nothing leaves this machine except to Claude</p></div>
      </div>
      <Show when={l()}>
        {(x) => (
          <div class="flex flex-col gap-2 border-y px-4 py-3">
            <Progress value={x().five_hour * 100} class="gap-1.5"><ProgressGroup><ProgressLabel class="text-xs font-normal text-muted-foreground">5-hour window · {left(x().five_hour_reset)}</ProgressLabel><ProgressValueLabel class="num text-xs" /></ProgressGroup></Progress>
            <Progress value={x().seven_day * 100} class="gap-1.5"><ProgressGroup><ProgressLabel class="text-xs font-normal text-muted-foreground">Week · {left(x().seven_day_reset)}</ProgressLabel><ProgressValueLabel class="num text-xs" /></ProgressGroup></Progress>
          </div>
        )}
      </Show>
      <div class="p-1.5">
        <Item icon={ic.sliders} label="Settings" hint="⌘," onClick={() => (props.onClose(), setView("settings"))} />
        <Item icon={ic.flask} label="Methods" onClick={() => (props.onClose(), setView("os"))} />
        <div class="flex h-9 items-center gap-2.5 rounded-lg px-2.5 text-sm">
          <Icon icon={ic.palette} size={16} class="text-muted-foreground" />Theme
          <div class="ml-auto flex rounded-full border p-0.5" role="radiogroup" aria-label="Theme">
            <For each={["system", "light", "dark"] as const}>{(t) => <button role="radio" aria-checked={theme() === t} onClick={() => setTheme(t)} class={cx("h-6 rounded-full px-2.5 text-xs capitalize transition-colors", theme() === t ? "bg-accent text-foreground" : "text-muted-foreground hover:text-foreground")}>{t}</button>}</For>
          </div>
        </div>
        <Item icon={ic.command} label="Search and actions" hint="Ctrl K" onClick={() => (props.onClose(), setPaletteOpen(true))} />
      </div>
      <p class="border-t px-4 py-2.5 text-xs text-muted-foreground">harnes {version() || ""}</p>
    </div>
  );
}

/** The ring's panel: the account's limits, where this project's tokens went, and compact. */
export function UsagePanel() {
  const p = () => stats()?.proxy;
  const orch = () => agents().find((a) => a.role === "orchestrator");
  const parts = () => {
    const x = p();
    if (!x) return [];
    return [
      { id: "input", label: "Input", n: x.input_tokens, tone: "bg-foreground/70" },
      { id: "cache", label: "Cache reads", n: x.cache_read_tokens, tone: "bg-foreground/35" },
      { id: "write", label: "Cache writes", n: x.cache_write_tokens, tone: "bg-foreground/20" },
      { id: "output", label: "Output", n: x.output_tokens, tone: "bg-foreground" },
    ];
  };
  const total = () => parts().reduce((s, x) => s + x.n, 0) || 1;
  const compact = async () => {
    try {
      await api.say("/compact");
      toast({ title: "Compacting", body: "The orchestrator is summarising its conversation.", tone: "ok" });
    } catch (e) {
      toast({ title: "Could not compact", body: String(e), tone: "danger" });
    }
  };
  return (
    <div class="flex w-[420px] flex-col gap-4 px-5 py-4">
      <div class="flex items-baseline justify-between text-sm">
        <p class="font-medium">Usage</p>
        <Show when={budget()?.eco}><Badge variant="secondary" class="font-normal">eco</Badge></Show>
      </div>
      <Show when={budget()?.limits} fallback={<p class="text-sm text-muted-foreground">Shown after the first agent request of the session.</p>}>
        {(l) => (
          <div class="flex flex-col gap-3">
            <Progress value={l().five_hour * 100} class="gap-1.5"><ProgressGroup><ProgressLabel class="text-xs font-normal text-muted-foreground">5-hour window · resets in {left(l().five_hour_reset)}</ProgressLabel><ProgressValueLabel class="num text-xs" /></ProgressGroup></Progress>
            <Progress value={l().seven_day * 100} class="gap-1.5"><ProgressGroup><ProgressLabel class="text-xs font-normal text-muted-foreground">Week · resets in {left(l().seven_day_reset)}</ProgressLabel><ProgressValueLabel class="num text-xs" /></ProgressGroup></Progress>
          </div>
        )}
      </Show>
      <Show when={p()}>
        <div class="flex flex-col gap-2 border-t pt-3">
          <div class="flex items-baseline justify-between text-sm"><p>Tokens this session</p><p class="num text-muted-foreground">{fmt(total() === 1 ? 0 : total())}</p></div>
          <div class="flex h-2 gap-px overflow-hidden rounded-full" role="img" aria-label="Where the tokens went">
            <For each={parts()}>{(x) => <div class={cx("h-full shrink-0", x.tone)} style={{ width: `${(x.n / total()) * 100}%` }} />}</For>
          </div>
          <ul class="grid grid-cols-2 gap-x-6 gap-y-1 text-sm">
            <For each={parts()}>{(x) => <li class="flex items-center gap-2"><span class={cx("size-2 rounded-[2px]", x.tone)} /><span class="text-muted-foreground">{x.label}</span><span class="num ml-auto font-medium">{fmt(x.n)}</span></li>}</For>
          </ul>
          <p class="num text-xs text-muted-foreground">{fmt(p()!.est_tokens_saved)} tokens saved by compressing tool results</p>
        </div>
      </Show>
      <div class="flex h-8 items-center justify-between border-t pt-3 text-sm">
        <p class="text-muted-foreground">{orch() ? (orch()!.state === "working" ? "The orchestrator is working" : "Summarise the conversation") : "No orchestrator running"}</p>
        <Button variant="secondary" size="sm" disabled={!orch() || orch()!.state === "working" || isRunning()} onClick={compact}>Compact now</Button>
      </div>
    </div>
  );
}
