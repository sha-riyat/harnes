// Methods: the packs of method the crew works with. A list on the left, the
// selected pack on the right. Changing the pack cross-fades the detail; roles are
// the same icons as everywhere else.

import { For, Show, createMemo, createSignal } from "solid-js";
import { Badge } from "@/components/ui/badge";
import { Card } from "@/components/ui/card";
import { cx } from "@/lib/cva";
import Icon from "../../ui/Icon";
import { ic } from "../icons";
import AgentIcon from "../ui/AgentIcon";
import { packText, packs } from "./viewData";

const tabs = ["Method", "Oracle", "Skills"] as const;

export default function MethodsView() {
  const [query, setQuery] = createSignal("");
  const [sel, setSel] = createSignal("builder-os");
  const [tab, setTab] = createSignal<(typeof tabs)[number]>("Method");
  const list = createMemo(() => packs.filter((p) => (p.id + p.purpose).toLowerCase().includes(query().toLowerCase())));
  const pack = () => packs.find((p) => p.id === sel())!;
  const text = () => (tab() === "Oracle" ? packText.oracle : packText.os);
  return (
    <div class="flex min-h-0 flex-1 flex-col gap-4 p-5">
      <header class="flex items-end gap-4 px-1">
        <div class="flex-1"><h1 class="text-2xl font-semibold tracking-tight">Methods</h1><p class="text-sm text-muted-foreground">{packs.length} packs the crew can work with. The roles that use a pack read it before they start.</p></div>
        <label class="flex h-9 w-72 items-center gap-2 rounded-lg border px-3 text-sm text-muted-foreground transition-colors focus-within:border-foreground/30"><Icon icon={ic.search} size={15} /><input class="min-w-0 flex-1 bg-transparent text-foreground outline-none placeholder:text-muted-foreground" placeholder="Filter packs" aria-label="Filter packs" value={query()} onInput={(e) => setQuery(e.currentTarget.value)} /></label>
      </header>
      <div class="grid min-h-0 flex-1 grid-cols-[360px_minmax(0,1fr)] gap-4">
        <Card class="min-h-0 gap-0 overflow-y-auto rounded-xl py-1.5 shadow-none">
          <For each={list()} fallback={<p class="px-4 py-8 text-center text-sm text-muted-foreground">No pack matches.</p>}>
            {(p, i) => (
              <button class={cx("m-rise relative flex flex-col gap-1.5 px-4 py-3 text-left transition-colors hover:bg-accent/50", sel() === p.id && "bg-accent")} style={{ "--i": i() }} onClick={(_id) => (setSel(p.id), setTab("Method"))} aria-current={sel() === p.id}>
                <Show when={sel() === p.id}><span class="absolute inset-y-2.5 left-0 w-0.5 rounded-full bg-foreground" /></Show>
                <span class="flex items-center gap-2 text-sm font-medium">{p.id}<span class="ml-auto font-mono text-xs font-normal text-muted-foreground">v0.1.0</span></span>
                <span class="line-clamp-2 text-xs leading-snug text-muted-foreground">{p.purpose}</span>
                <span class="flex items-center gap-1.5">
                  <For each={p.roles} fallback={<span class="text-xs text-muted-foreground/70">No role yet</span>}>{(r) => <AgentIcon role={r} size="sm" />}</For>
                  <span class="num ml-auto text-xs text-muted-foreground">{p.skills.length} skills</span>
                </span>
              </button>
            )}
          </For>
        </Card>
        <Card class="min-h-0 gap-0 overflow-hidden rounded-xl py-0 shadow-none">
          <Show when={sel()} keyed>
            {(_id) => (
              <div class="m-rise flex min-h-0 flex-1 flex-col">
                <header class="flex items-start gap-3 border-b px-5 py-4">
                  <div class="flex-1"><h2 class="text-lg font-semibold">{pack().id}</h2><p class="mt-1 max-w-xl text-sm text-muted-foreground">{pack().purpose}</p></div>
                  <Badge variant="outline" class="font-normal" title="How sure the pack is of its own advice">Confidence {pack().confidence}</Badge>
                </header>
                <div class="flex gap-5 border-b px-5" role="tablist">
                  <For each={tabs}>
                    {(t) => <button role="tab" aria-selected={tab() === t} onClick={() => setTab(t)} class={cx("relative pt-3 pb-2.5 text-sm transition-colors", tab() === t ? "text-foreground" : "text-muted-foreground hover:text-foreground")}>{t}<Show when={tab() === t}><span class="m-pop absolute inset-x-0 -bottom-px h-0.5 bg-foreground" /></Show></button>}
                  </For>
                </div>
                <div class="min-h-0 flex-1 overflow-y-auto px-5 py-4">
                  <Show when={tab() !== "Skills"} fallback={
                    <ul class="grid grid-cols-2 gap-2">
                      <For each={pack().skills}>{(s, i) => <li class="m-rise flex h-11 items-center gap-2.5 rounded-lg border px-3 text-sm" style={{ "--i": i() }}><Icon icon={ic.lightning} size={15} class="text-muted-foreground" /><span class="font-mono text-[13px]">{s}</span></li>}</For>
                    </ul>
                  }>
                    <div class="flex max-w-xl flex-col gap-2 text-sm leading-relaxed">
                      <For each={text()}>{(l, i) => <p class={cx("m-rise", i() % 2 === 0 ? "pt-2 text-xs font-medium text-muted-foreground" : "")} style={{ "--i": i() }}>{l}</p>}</For>
                    </div>
                  </Show>
                </div>
              </div>
            )}
          </Show>
        </Card>
      </div>
    </div>
  );
}
