// Methods: the OS packs the crew applies. Each role loads its packs; the rest are
// reached on demand. A list on the left, the selected pack on the right (its doctrine,
// its oracle and its skills), plus import. Read-only apart from import.

import { open } from "@tauri-apps/plugin-dialog";
import { For, Show, createMemo, createResource, createSignal } from "solid-js";
import { api, type OsPack } from "../../api";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { cx } from "@/lib/cva";
import Icon from "../../ui/Icon";
import AgentIcon from "../../ui/kit/AgentIcon";
import type { Role } from "../../ui/kit/roles";
import Markdown from "../../ui/Markdown";
import { ic } from "../../ui/ph";
import { toast } from "../../ui/state";

const TABS = ["Method", "Oracle", "Skills"] as const;
const short = (id: string) => id.replace(/-os$/, "");
const ROLES = ["orchestrator", "planner", "teacher", "worker", "scout", "reviewer"];

async function importPacks(dir: boolean, refetch: () => void) {
  const picked = await open(dir ? { directory: true, title: "Folder of OS packs" } : { multiple: false, title: "OS pack (.zip)", filters: [{ name: "zip", extensions: ["zip"] }] });
  if (!picked || Array.isArray(picked)) return;
  try {
    const ids = await api.osImport(picked);
    toast({ title: `${ids.length} pack${ids.length === 1 ? "" : "s"} imported`, body: "Open projects already see them.", tone: "ok" });
    refetch();
  } catch (e) {
    toast({ title: "Import failed", body: String(e), tone: "danger" });
  }
}

function Detail(props: { id: string }) {
  const [d] = createResource(() => props.id, (id) => api.osPack(id).catch(() => null));
  const [tab, setTab] = createSignal<(typeof TABS)[number]>("Method");
  const [skill, setSkill] = createSignal<string | null>(null);
  const [skillText] = createResource(skill, (s) => api.osRead(props.id, `skills/${s}/SKILL.md`).catch(() => ""));
  return (
    <Show when={d()} fallback={<p class="m-auto text-sm text-muted-foreground">Reading the pack…</p>}>
      {(x) => (
        <div class="m-rise flex min-h-0 flex-1 flex-col">
          <header class="flex items-start gap-3 border-b px-5 py-4">
            <div class="flex-1"><h2 class="text-lg font-semibold">{x().pack.id}</h2><p class="mt-1 max-w-xl text-sm text-muted-foreground">{x().pack.purpose}</p></div>
            <Badge variant="outline" class="font-normal" title="How sure the pack is of its own advice">Confidence {x().pack.confidence}</Badge>
          </header>
          <div class="flex gap-5 border-b px-5" role="tablist">
            <For each={TABS}>{(t) => <button role="tab" aria-selected={tab() === t} onClick={() => setTab(t)} class={cx("relative pt-3 pb-2.5 text-sm transition-colors", tab() === t ? "text-foreground" : "text-muted-foreground hover:text-foreground")}>{t}<Show when={tab() === t}><span class="m-pop absolute inset-x-0 -bottom-px h-0.5 bg-foreground" /></Show></button>}</For>
          </div>
          <div class="min-h-0 flex-1 overflow-y-auto px-5 py-4">
            <Show when={tab() === "Method"}><div class="m-rise max-w-2xl text-sm leading-relaxed"><Markdown text={x().os} /></div></Show>
            <Show when={tab() === "Oracle"}><div class="m-rise max-w-2xl text-sm leading-relaxed"><Markdown text={x().oracle} /></div></Show>
            <Show when={tab() === "Skills"}>
              <ul class="grid grid-cols-2 gap-2">
                <For each={x().pack.skills}>{(s, i) => <li class="m-rise" style={{ "--i": i() }}><button class={cx("flex h-11 w-full items-center gap-2.5 rounded-lg border px-3 text-sm transition-colors hover:bg-accent/50", skill() === s && "border-foreground")} onClick={() => setSkill(skill() === s ? null : s)}><Icon icon={ic.lightning} size={15} class="text-muted-foreground" /><span class="font-mono text-[13px]">{s}</span></button></li>}</For>
              </ul>
              <Show when={skill() && skillText()}><div class="m-rise mt-4 max-w-2xl rounded-lg border p-4 text-sm leading-relaxed"><Markdown text={skillText()!} /></div></Show>
            </Show>
          </div>
        </div>
      )}
    </Show>
  );
}

export default function Methods() {
  const [lib, { refetch }] = createResource(() => api.osList().catch(() => ({ dir: "", packs: [] as OsPack[], offline: true })));
  const [query, setQuery] = createSignal("");
  const [sel, setSel] = createSignal<string | null>(null);
  const packs = () => lib()?.packs ?? [];
  const list = createMemo(() => packs().filter((p) => (p.id + p.purpose).toLowerCase().includes(query().toLowerCase())));
  const current = () => sel() ?? list()[0]?.id ?? null;
  return (
    <div class="flex min-h-0 flex-1 flex-col gap-4 p-5">
      <header class="flex items-end gap-3 px-1">
        <div class="flex-1"><h1 class="text-2xl font-semibold tracking-tight">Methods</h1><p class="text-sm text-muted-foreground">{packs().length ? `${packs().length} packs the crew can work with. The roles that use a pack read it before they start.` : (lib() as { offline?: boolean })?.offline ? "The backend is offline: open the harnes app." : "No pack yet."}</p></div>
        <label class="flex h-9 w-64 items-center gap-2 rounded-lg border px-3 text-sm text-muted-foreground transition-colors focus-within:border-foreground/30"><Icon icon={ic.search} size={15} /><input class="min-w-0 flex-1 bg-transparent text-foreground outline-none placeholder:text-muted-foreground" placeholder="Filter packs" aria-label="Filter packs" value={query()} onInput={(e) => setQuery(e.currentTarget.value)} /></label>
        <Button variant="outline" size="sm" class="h-9" onClick={() => importPacks(true, refetch)}><Icon icon={ic.folderPlus} size={15} />Import a folder</Button>
        <Button variant="outline" size="sm" class="h-9" onClick={() => importPacks(false, refetch)}><Icon icon={ic.upload} size={15} />Import a zip</Button>
      </header>
      <Show when={packs().length} fallback={<Card class="m-auto max-w-md gap-2 rounded-xl px-6 py-8 text-center shadow-none"><p class="font-medium">A pack is a complete method for agents</p><p class="text-sm text-muted-foreground">Doctrine, oracle, skills and routing. Import a folder of zips or one pack: each agent of the crew loads the ones of its role.</p></Card>}>
        <div class="grid min-h-0 flex-1 grid-cols-[360px_minmax(0,1fr)] gap-4">
          <Card class="min-h-0 gap-0 overflow-y-auto rounded-xl py-1.5 shadow-none">
            <For each={list()} fallback={<p class="px-4 py-8 text-center text-sm text-muted-foreground">No pack matches.</p>}>
              {(p, i) => (
                <button class={cx("m-rise relative flex flex-col gap-1.5 px-4 py-3 text-left transition-colors hover:bg-accent/50", current() === p.id && "bg-accent")} style={{ "--i": i() }} aria-current={current() === p.id} onClick={() => setSel(p.id)}>
                  <Show when={current() === p.id}><span class="absolute inset-y-2.5 left-0 w-0.5 rounded-full bg-foreground" /></Show>
                  <span class="flex items-center gap-2 text-sm font-medium">{short(p.id)}<span class="ml-auto font-mono text-xs font-normal text-muted-foreground">v{p.version}</span></span>
                  <span class="line-clamp-2 text-xs leading-snug text-muted-foreground">{p.purpose}</span>
                  <span class="flex items-center gap-1.5"><For each={p.roles.filter((r) => ROLES.includes(r))} fallback={<span class="text-xs text-muted-foreground/70">On demand</span>}>{(r) => <AgentIcon role={r as Role} size="sm" />}</For><span class="num ml-auto text-xs text-muted-foreground">{p.skills.length} skills</span></span>
                </button>
              )}
            </For>
            <p class="px-4 py-2 font-mono text-[11px] break-all text-muted-foreground/70">{lib()?.dir}</p>
          </Card>
          <Card class="min-h-0 gap-0 overflow-hidden rounded-xl py-0 shadow-none"><Show when={current()} keyed>{(id) => <Detail id={id} />}</Show></Card>
        </div>
      </Show>
    </div>
  );
}
