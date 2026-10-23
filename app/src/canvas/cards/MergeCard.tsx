// A pull request from open to merged (after the "Merge conflicts" reference).
// Main moved while the lot was built: two conflicts show, are opened, then
// resolved one by one; the merge commit rises on top of the branch, checks tick
// in turn, and the Merge button lands the branch on main.

import { For, Match, Show, Switch } from "solid-js";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { cx } from "@/lib/cva";
import Icon from "../../ui/Icon";
import { ic } from "../icons";
import { createSim } from "../ui/sim";
import { pr } from "./cardData";

type Stage = "conflict" | "open" | "resolving" | "checks" | "ready" | "merging" | "merged";
// Tick at which each stage begins.
const STAGES: [Stage, number][] = [["merged", 14], ["merging", 12], ["ready", 10], ["checks", 7], ["resolving", 4], ["open", 2], ["conflict", 0]];
const stageAt = (t: number): Stage => STAGES.find(([, from]) => t >= from)![0];

function Check(props: { on: boolean; running: boolean; label: string }) {
  return (
    <span class="flex items-center gap-1.5 text-xs text-muted-foreground">
      <span class="grid size-3.5 place-items-center">
        <Switch>
          <Match when={props.on}><svg viewBox="0 0 16 16" class="size-3.5 text-foreground"><path d="M3.5 8.5l3 3 6-7" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" class="m-draw" style={{ "--len": "16", "--dur": "200ms" }} /></svg></Match>
          <Match when={props.running}><span class="m-breathe size-1.5 rounded-full bg-foreground" /></Match>
          <Match when={true}><span class="size-1.5 rounded-full border border-muted-foreground/60" /></Match>
        </Switch>
      </span>
      <span class={cx(props.on && "text-foreground")}>{props.label}</span>
    </span>
  );
}

export default function MergeCard() {
  const sim = createSim(1000, 16, 4);
  const t = sim.t;
  const stage = () => stageAt(t());
  const resolved = () => (t() >= 6 ? 2 : t() >= 5 ? 1 : 0);
  const clear = () => t() >= 6; // no conflict left from tick 6
  const merged = () => stage() === "merged";
  const commits = () => (clear() ? [pr.mergeCommit, ...pr.commits] : pr.commits);
  return (
    <Card class="gap-0 rounded-xl py-0 shadow-none">
      <header class="flex items-center gap-2.5 border-b px-4 py-3">
        <Icon icon={merged() ? ic.merge : ic.pullRequest} size={18} class={cx(merged() ? "text-foreground" : "text-muted-foreground")} />
        <div class="min-w-0 flex-1 leading-tight"><p class="truncate text-sm font-medium"><span class="num text-muted-foreground">#{pr.n}</span> {pr.title}</p><p class="font-mono text-xs text-muted-foreground">{pr.branch} → {pr.base}</p></div>
        <Badge variant={merged() ? "default" : "outline"} class="font-normal transition-colors duration-300">{merged() ? "Merged" : "Open"}</Badge>
      </header>

      <ol class="flex flex-col px-4 pt-3">
        <For each={commits()}>
          {(c, i) => (
            <li class={cx("relative flex items-center gap-3 rounded-md py-1.5", c === pr.mergeCommit && "m-rise m-wash")} style={{ "--i": 0 }}>
              <Show when={i() < commits().length - 1}><span class="absolute top-[22px] -bottom-[8px] left-[7.5px] w-px bg-border" /></Show>
              <span class="grid size-4 shrink-0 place-items-center">
                <span class={cx("size-2 rounded-full border-2 bg-background", c === pr.mergeCommit ? "border-foreground" : "base" in c ? "border-muted-foreground/40" : "border-muted-foreground")} />
              </span>
              <span class={cx("min-w-0 flex-1 truncate text-sm", "base" in c && "text-muted-foreground")}>{c.msg}</span>
              <span class="font-mono text-xs text-muted-foreground">{c.sha}</span>
              <span class="num w-32 truncate text-right text-xs text-muted-foreground">{c.who} · {c.at}</span>
            </li>
          )}
        </For>
      </ol>

      <div class="m-fold mt-1 border-t" data-folded={clear()}>
        <div>
          <div class="flex flex-col gap-2 px-4 py-3">
            <p class="flex items-center gap-2 text-sm"><span class="m-ring size-1.5 rounded-full bg-signal" /><span class="font-medium">{pr.conflicts.length} conflicts</span> <span class="text-muted-foreground">with {pr.base}</span>
              <Show when={stage() === "conflict" || stage() === "open"}><Button size="sm" variant="outline" class="ml-auto h-7">Resolve conflicts</Button></Show>
              <Show when={stage() === "resolving"}><span class="m-breathe ml-auto text-xs text-muted-foreground">Resolving conflict {Math.min(resolved() + 1, 2)} of 2</span></Show>
            </p>
            <div class="m-fold" data-folded={stage() === "conflict"}>
              <div>
                <ul class="flex flex-col gap-1.5 pb-1">
                  <For each={pr.conflicts}>
                    {(c, i) => (
                      <li class={cx("rounded-md border px-3 py-2 font-mono text-xs transition-opacity duration-300", resolved() > i() && "opacity-40")}>
                        <p class="mb-1 flex items-center justify-between text-muted-foreground"><span>{c.file} · line {c.line}</span><Show when={resolved() > i()}><span class="m-pop inline-flex items-center gap-1 font-sans"><Icon icon={ic.checkBold} size={11} />resolved</span></Show></p>
                        <p class="text-muted-foreground">main  {c.theirs}</p>
                        <p>lot   {c.ours}</p>
                      </li>
                    )}
                  </For>
                </ul>
              </div>
            </div>
          </div>
        </div>
      </div>

      <Show when={clear()}>
        <div class="m-rise flex flex-wrap items-center gap-x-4 gap-y-1.5 border-t px-4 py-3">
          <For each={pr.checks}>{(c, i) => <Check label={c} on={t() >= 7 + i() + 1} running={t() === 7 + i()} />}</For>
        </div>
      </Show>

      <footer class="flex h-12 items-center gap-3 border-t px-4 text-sm">
        <Switch>
          <Match when={stage() === "merged"}><span class="m-rise inline-flex items-center gap-2 font-medium"><Icon icon={ic.merge} size={16} />Merged into {pr.base}</span></Match>
          <Match when={stage() === "merging"}><span class="m-breathe text-muted-foreground">Merging into {pr.base}</span></Match>
          <Match when={stage() === "ready"}><span class="m-rise inline-flex items-center gap-2"><Icon icon={ic.checkBold} size={14} />Ready to merge</span></Match>
          <Match when={stage() === "checks"}><span class="m-breathe text-muted-foreground">Running checks</span></Match>
          <Match when={true}><span class="text-muted-foreground">{clear() ? "No conflicts" : `${pr.conflicts.length - resolved()} conflicts to resolve`}</span></Match>
        </Switch>
        <Button size="sm" class="ml-auto h-8 transition-opacity" disabled={stage() !== "ready"}>
          <Icon icon={ic.merge} size={15} />Merge
        </Button>
      </footer>
    </Card>
  );
}
