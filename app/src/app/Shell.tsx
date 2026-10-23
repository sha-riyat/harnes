// The frame of the app: a tinted L (top bar and sidebar) around one white sheet. The
// sheet holds a project's title and tabs, or Methods, or Settings, and docks a terminal
// at its foot on demand. The bell and the avatar open panels in place.

import { For, Match, Show, Switch, createSignal } from "solid-js";
import { api } from "../api";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Kbd } from "@/components/ui/kbd";
import { Tabs, TabsIndicator, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cx } from "@/lib/cva";
import { isRunning, needCount, unreadCount } from "../model";
import { currentProject, info, setView, view } from "../store";
import Icon from "../ui/Icon";
import { Mark } from "../ui/kit/bits";
import Pop from "../ui/kit/Pop";
import { ic } from "../ui/ph";
import { addProject } from "../ui/projects";
import { setPaletteOpen, setSidebarOpen, sidebarOpen } from "../ui/state";
import { NotificationsPanel, ProfileMenu } from "./Panels";
import Sidebar from "./Sidebar";
import Agents from "./views/Agents";
import Chat from "./views/Chat";
import Methods from "./views/Methods";
import Overview from "./views/Overview";
import Run from "./views/Run";
import Settings from "./views/Settings";
import Terminals, { TermDock } from "./views/Terminals";

const TABS = [["overview", "Overview"], ["chat", "Chat"], ["run", "Run"], ["agents", "Agents"], ["panes", "Terminals"]] as const;
const inProject = () => ["overview", "chat", "run", "agents", "panes"].includes(view());

function TopBar() {
  const [panel, setPanel] = createSignal<"notices" | "profile" | null>(null);
  return (
    <header class="flex h-12 shrink-0 items-center gap-3 px-3">
      <div class="flex w-[212px] items-center gap-1 px-1">
        <Button variant="ghost" size="icon" class={cx("size-8 text-muted-foreground", sidebarOpen() && "text-foreground")} aria-label="Toggle the sidebar" title="Sidebar" onClick={() => setSidebarOpen(!sidebarOpen())}><Icon icon={ic.sidebarSimple} size={17} /></Button>
        <span class="flex items-center gap-2 font-semibold"><Mark class="size-5" />harnes</span>
      </div>
      <Button variant="outline" class="mx-auto h-8 w-[548px] max-w-full justify-start gap-2 bg-background/60 font-normal text-muted-foreground shadow-none" onClick={() => setPaletteOpen(true)}>
        <Icon icon={ic.search} size={15} />Search actions, projects, terminals<Kbd class="ml-auto">Ctrl K</Kbd>
      </Button>
      <div class="relative">
        <Button variant="ghost" size="icon" aria-label="Notifications" aria-expanded={panel() === "notices"} class={cx("relative size-8 text-muted-foreground", panel() === "notices" && "bg-accent text-foreground")} onClick={() => setPanel(panel() === "notices" ? null : "notices")}>
          <Icon icon={ic.bell} size={17} />
          <Show when={unreadCount() > 0}><span class="m-pop absolute top-1.5 right-1.5 size-1.5 rounded-full bg-signal" /></Show>
        </Button>
        <Pop open={panel() === "notices"} onClose={() => setPanel(null)} align="right" label="Notifications"><NotificationsPanel onClose={() => setPanel(null)} /></Pop>
      </div>
      <div class="relative">
        <button aria-label="Profile" aria-expanded={panel() === "profile"} class="rounded-full ring-offset-2 ring-offset-[var(--frame)] transition-shadow hover:ring-2 hover:ring-foreground/15 aria-expanded:ring-2 aria-expanded:ring-foreground/25" onClick={() => setPanel(panel() === "profile" ? null : "profile")}>
          <span class="grid size-7 place-items-center rounded-full bg-secondary text-[11px] font-medium text-secondary-foreground">h</span>
        </button>
        <Pop open={panel() === "profile"} onClose={() => setPanel(null)} align="right" label="Profile"><ProfileMenu onClose={() => setPanel(null)} /></Pop>
      </div>
    </header>
  );
}

function ProjectHeader() {
  return (
    <div class="px-6 pt-5">
      <div class="flex items-end gap-4">
        <div class="min-w-0 flex-1">
          <Show when={currentProject()} fallback={<h1 class="text-2xl font-semibold tracking-tight">No project</h1>}>
            {(p) => (
              <>
                <p class="text-xs text-muted-foreground">Projects</p>
                <div class="flex items-center gap-3">
                  <h1 class="truncate text-2xl font-semibold tracking-tight">{p().name}</h1>
                  <Switch>
                    <Match when={needCount() > 0}><Badge variant="outline" class="gap-1.5 text-foreground"><span class="m-ring size-1.5 rounded-full bg-signal" />Needs you</Badge></Match>
                    <Match when={isRunning()}><Badge variant="outline" class="gap-1.5 font-normal text-muted-foreground"><span class="m-breathe size-1.5 rounded-full bg-foreground" />Running</Badge></Match>
                  </Switch>
                </div>
                <p class="mt-0.5 truncate font-mono text-xs text-muted-foreground">{p().root}</p>
              </>
            )}
          </Show>
        </div>
        <Show when={isRunning()}><Button variant="outline" onClick={() => api.crewCancel()}>Stop the run</Button></Show>
        <Show when={!info()?.project}><Button onClick={addProject}>Add a project</Button></Show>
      </div>
      <Tabs value={view()} onChange={(v) => setView(v as ReturnType<typeof view>)} class="mt-3 gap-0">
        <TabsList class="h-auto w-full justify-start gap-5 rounded-none border-b bg-transparent p-0 ring-0">
          <For each={TABS}>{([id, label]) => <TabsTrigger value={id} class="flex-none px-0 pt-1 pb-2.5 text-muted-foreground data-[selected]:text-foreground dark:data-[selected]:text-foreground">{label}</TabsTrigger>}</For>
          <TabsIndicator class="inset-x-auto top-auto bottom-[-1px] h-0.5 rounded-none border-0 bg-foreground shadow-none dark:bg-foreground" />
        </TabsList>
      </Tabs>
    </div>
  );
}

export default function Shell() {
  return (
    <div class="flex h-full w-full flex-col overflow-hidden bg-[var(--frame)] text-foreground">
      <TopBar />
      <div class="flex min-h-0 flex-1">
        <Show when={sidebarOpen()}><Sidebar /></Show>
        <section class={cx("mr-2 mb-2 flex min-w-0 flex-1 flex-col overflow-hidden rounded-2xl border bg-background", !sidebarOpen() && "ml-2")}>
          <Show when={inProject()}><ProjectHeader /></Show>
          <div class="flex min-h-0 flex-1 flex-col">
            <Switch>
              <Match when={view() === "overview"}><Overview /></Match>
              <Match when={view() === "chat"}><Chat /></Match>
              <Match when={view() === "run"}><Run /></Match>
              <Match when={view() === "agents"}><Agents /></Match>
              <Match when={view() === "panes"}><Terminals /></Match>
              <Match when={view() === "os"}><Methods /></Match>
              <Match when={view() === "settings"}><Settings /></Match>
            </Switch>
          </div>
          <TermDock />
        </section>
      </div>
    </div>
  );
}
