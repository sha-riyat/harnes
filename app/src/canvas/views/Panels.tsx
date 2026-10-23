// The two panels under the top bar: notifications (the bell) and the profile menu
// (the avatar). Both open in place, close on Escape or a press outside, and keep
// the same anatomy as the rest: quiet rows, one action on the row that needs it.

import { For, Show, createSignal } from "solid-js";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Kbd } from "@/components/ui/kbd";
import { Progress, ProgressGroup, ProgressLabel, ProgressValueLabel } from "@/components/ui/progress";
import { cx } from "@/lib/cva";
import Icon from "../../ui/Icon";
import { minutes, usage } from "../data";
import { ic } from "../icons";
import AgentIcon from "../ui/AgentIcon";
import { Initials } from "../ui/parts";
import { liveNotice, notices, profile, type Notice } from "./viewData";

export function NotificationsPanel(props: { onOpenSettings?: () => void; live?: boolean; onAnswer?: () => void }) {
  const [filter, setFilter] = createSignal<"all" | "ask">("all");
  const [read, setRead] = createSignal<string[]>([]);
  const list = (): Notice[] => [...(props.live ? [liveNotice] : []), ...notices];
  const isUnread = (n: Notice) => n.unread && !read().includes(n.id);
  const shown = () => list().filter((n) => filter() === "all" || n.ask);
  const unread = () => list().filter(isUnread).length;
  return (
    <div class="w-[400px]">
      <header class="flex items-center gap-2 px-4 pt-3.5 pb-2">
        <h2 class="text-sm font-semibold">Notifications</h2>
        <Show when={unread()} keyed>{(n) => <Badge variant="secondary" class="num m-tick">{n}</Badge>}</Show>
        <Button variant="ghost" size="sm" class="ml-auto h-7 text-muted-foreground" disabled={!unread()} onClick={() => setRead(list().map((n) => n.id))}>Mark all read</Button>
      </header>
      <div class="flex gap-1 px-3 pb-2" role="tablist">
        <For each={[["all", "All"], ["ask", "Needs you"]] as const}>
          {([id, label]) => (
            <button role="tab" aria-selected={filter() === id} onClick={() => setFilter(id)} class={cx("h-7 rounded-full px-3 text-xs transition-colors", filter() === id ? "bg-accent text-foreground" : "text-muted-foreground hover:text-foreground")}>{label}</button>
          )}
        </For>
      </div>
      <ul class="flex max-h-[360px] flex-col overflow-y-auto border-t px-1.5 py-1.5">
        <For each={shown()} fallback={<li class="m-rise px-4 py-8 text-center text-sm text-muted-foreground">Nothing waits for you.</li>}>
          {(n) => (
            <li class={cx("flex gap-3 rounded-lg px-2.5 py-2.5 transition-colors hover:bg-accent/50", n.live && "m-rise m-wash")}>
              <AgentIcon role={n.role} state={n.ask ? "blocked" : undefined} size="md" />
              <div class="min-w-0 flex-1 text-sm leading-snug">
                <p><span class="font-medium">{n.who}</span> <span class="text-muted-foreground">{n.text}</span></p>
                <p class="num mt-0.5 text-xs text-muted-foreground">{n.at}</p>
              </div>
              <div class="flex shrink-0 flex-col items-end gap-1.5">
                <Show when={isUnread(n)}><span class={cx("m-pop mt-1.5 size-1.5 rounded-full", n.ask ? "bg-signal" : "bg-foreground")} /></Show>
                <Show when={n.action}><Button variant="outline" size="sm" class="h-7" onClick={props.onAnswer}>{n.action}</Button></Show>
              </div>
            </li>
          )}
        </For>
      </ul>
      <footer class="flex items-center border-t px-4 py-2.5 text-sm">
        <button class="inline-flex items-center gap-2 text-muted-foreground transition-colors hover:text-foreground" onClick={props.onOpenSettings}><Icon icon={ic.gear} size={15} />Notification settings</button>
      </footer>
    </div>
  );
}

function Item(props: { icon: string; label: string; hint?: string; danger?: boolean; onClick?: () => void; children?: any }) {
  return (
    <button class="flex h-9 w-full items-center gap-2.5 rounded-lg px-2.5 text-sm transition-colors hover:bg-accent" onClick={props.onClick}>
      <Icon icon={props.icon} size={16} class="text-muted-foreground" />{props.label}
      <span class="ml-auto text-xs text-muted-foreground">{props.hint}</span>{props.children}
    </button>
  );
}

export function ProfileMenu(props: { onSettings?: () => void; theme: string; onTheme: (t: "system" | "light" | "dark") => void }) {
  return (
    <div class="w-[300px]">
      <div class="flex items-center gap-3 px-4 py-3.5">
        <Initials name="Sha Riyat" class="size-10 text-xs" />
        <div class="min-w-0 flex-1 leading-tight"><p class="text-sm font-semibold">{profile.name}</p><p class="text-xs text-muted-foreground">{profile.plan} · {profile.via}</p></div>
      </div>
      <div class="flex flex-col gap-2 border-y px-4 py-3">
        <Progress value={usage.fiveHour * 100} class="gap-1.5"><ProgressGroup><ProgressLabel class="text-xs font-normal text-muted-foreground">5-hour window · {minutes(usage.resetsIn)}</ProgressLabel><ProgressValueLabel class="num text-xs" /></ProgressGroup></Progress>
        <Progress value={usage.week * 100} class="gap-1.5"><ProgressGroup><ProgressLabel class="text-xs font-normal text-muted-foreground">Week · {usage.weekResetsInDays} d</ProgressLabel><ProgressValueLabel class="num text-xs" /></ProgressGroup></Progress>
      </div>
      <div class="p-1.5">
        <Item icon={ic.userCircle} label="Claude account" hint="Manage" />
        <Item icon={ic.sliders} label="Settings" hint="⌘," onClick={props.onSettings} />
        <div class="flex h-9 items-center gap-2.5 rounded-lg px-2.5 text-sm">
          <Icon icon={ic.palette} size={16} class="text-muted-foreground" />Theme
          <div class="ml-auto flex rounded-full border p-0.5" role="radiogroup" aria-label="Theme">
            <For each={["system", "light", "dark"] as const}>{(t) => <button role="radio" aria-checked={props.theme === t} onClick={() => props.onTheme(t)} class={cx("h-6 rounded-full px-2.5 text-xs capitalize transition-colors", props.theme === t ? "bg-accent text-foreground" : "text-muted-foreground hover:text-foreground")}>{t}</button>}</For>
          </div>
        </div>
        <Item icon={ic.translate} label="Language" hint="English" />
        <Item icon={ic.keyboard} label="Keyboard shortcuts"><Kbd class="ml-1.5">⌘/</Kbd></Item>
      </div>
      <p class="border-t px-4 py-2.5 text-xs text-muted-foreground">{profile.version}</p>
    </div>
  );
}
