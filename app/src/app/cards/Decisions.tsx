// What waits for the user, as cards: a typed question (ask_user), a permission, a
// question read off an agent's terminal, and the trust scan of the project's config.
// Answering folds the card to one line and then lets it go; the agent's tile in the
// sidebar goes back to "working" on its own when its hook reports it.

import { For, Show, createResource, createSignal, type JSX } from "solid-js";
import { api, type AskPending, type PermissionAsk, type ScreenPrompt, type TrustFinding } from "../../api";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { RadioGroup, RadioGroupItem, RadioGroupItemControl, RadioGroupItemIndicator, RadioGroupItemInput, RadioGroupItemLabel, RadioGroupItems } from "@/components/ui/radio-group";
import { cx } from "@/lib/cva";
import { waitingAsks, waitingPermissions, waitingScreens } from "../../model";
import { panes, setAsks, setDrawerPane, setPermissions, setScreenPrompts } from "../../store";
import Icon from "../../ui/Icon";
import AgentIcon from "../../ui/kit/AgentIcon";
import { nameOf, roleOf } from "../../ui/kit/roles";
import { ic } from "../../ui/ph";
import { toast } from "../../ui/state";
import { ago } from "../Panels";

const who = (key: string) => ({ name: nameOf(key), role: roleOf(key, panes.find((p) => p.key === key)?.role) });
const FOLD_MS = 1800;

/** Shared frame: the header with the agent, a body that folds, a one-line result. */
function Frame(props: { agent: string; verb: string; meta: string; done: string | null; chip?: JSX.Element; children: JSX.Element }) {
  const w = () => who(props.agent);
  return (
    <Card class="m-rise gap-0 rounded-xl py-0 shadow-none">
      <header class="flex items-center gap-2.5 px-4 py-3">
        <AgentIcon role={w().role} state={props.done ? "working" : "blocked"} />
        <div class="min-w-0 flex-1 leading-tight">
          <p class="truncate text-sm font-medium">{w().name} <span class="font-normal text-muted-foreground">{props.verb}</span></p>
          <p class="num text-xs text-muted-foreground">{props.meta}</p>
        </div>
        <Show when={!props.done} fallback={<span class="m-pop inline-flex items-center gap-1 text-xs text-muted-foreground"><Icon icon={ic.checkBold} size={12} />Answered</span>}>
          {props.chip ?? <span class="inline-flex items-center gap-1.5 text-xs text-muted-foreground"><span class="m-ring size-1.5 rounded-full bg-signal" />waiting</span>}
        </Show>
      </header>
      <div class="m-fold" data-folded={!!props.done}><div>{props.children}</div></div>
      <Show when={props.done}><p class="m-rise border-t px-4 py-2.5 text-sm text-muted-foreground">{props.done}</p></Show>
    </Card>
  );
}

const fail = (e: unknown) => toast({ title: "Answer not delivered", body: String(e), tone: "danger" });

export function QuestionCard(props: { ask: AskPending }) {
  const c = () => props.ask.card;
  const [done, setDone] = createSignal<string | null>(null);
  const [pick, setPick] = createSignal(c().default ?? c().options[0]?.id ?? "");
  const [text, setText] = createSignal("");
  const release = () => setTimeout(() => setAsks((l) => l.filter((a) => a.id !== props.ask.id)), FOLD_MS);
  const send = async (choice?: string, free?: string, label?: string) => {
    try {
      await api.askAnswer(props.ask.id, choice, free);
      setDone(`You answered ${label ?? (free ? "in words" : choice)}. The agent carries on.`);
      release();
    } catch (e) {
      fail(e);
      if (String(e).includes("expir")) setAsks((l) => l.filter((a) => a.id !== props.ask.id));
    }
  };
  const skip = async () => {
    await api.askDecline(props.ask.id).catch(() => {});
    setDone("You skipped it. The agent decides alone, or takes its default.");
    release();
  };
  return (
    <Frame agent={props.ask.agent} verb="asks" meta={ago(props.ask.at)} done={done()}>
      <div class="flex flex-col gap-3 px-4 pb-4">
        <p class="text-sm leading-snug font-medium">{c().question}</p>
        <Show when={c().kind === "choice"}>
          <RadioGroup value={pick()} onChange={setPick} aria-label="Answer">
            <RadioGroupItems class="grid grid-cols-2 gap-2">
              <For each={c().options}>
                {(o) => (
                  <RadioGroupItem value={o.id} class="rounded-lg border px-3 py-2.5 transition-[border-color,background-color] duration-150 hover:bg-accent data-[checked]:border-foreground">
                    <RadioGroupItemInput />
                    <RadioGroupItemControl><RadioGroupItemIndicator /></RadioGroupItemControl>
                    <RadioGroupItemLabel class="flex flex-col gap-0 leading-tight">{o.label}<Show when={o.detail}><span class="text-xs font-normal text-muted-foreground">{o.detail}</span></Show></RadioGroupItemLabel>
                  </RadioGroupItem>
                )}
              </For>
            </RadioGroupItems>
          </RadioGroup>
        </Show>
        <Show when={c().kind === "text"}>
          <input class="h-9 rounded-lg border bg-transparent px-3 text-sm outline-none focus-visible:border-foreground/30 focus-visible:ring-[3px] focus-visible:ring-foreground/5" maxLength={2000} placeholder={c().default ?? "Your answer"} aria-label="Answer" value={text()} onInput={(e) => setText(e.currentTarget.value)} onKeyDown={(e) => e.key === "Enter" && text().trim() && send(undefined, text())} />
        </Show>
        <div class="flex flex-wrap items-center gap-2">
          <Show when={c().kind === "choice"}><Button size="sm" onClick={() => send(pick(), undefined, c().options.find((o) => o.id === pick())?.label)}>Send answer</Button></Show>
          <Show when={c().kind === "confirm"}>
            <Button size="sm" onClick={() => send("yes", undefined, "yes")}>Yes</Button>
            <Button size="sm" variant="outline" onClick={() => send("no", undefined, "no")}>No</Button>
          </Show>
          <Show when={c().kind === "text"}><Button size="sm" disabled={!text().trim()} onClick={() => send(undefined, text())}>Send</Button></Show>
          <Button size="sm" variant="ghost" class="text-muted-foreground" title="The agent carries on with its default, or decides alone" onClick={skip}>Skip</Button>
          <Show when={c().default}><span class="ml-auto text-xs text-muted-foreground">If there is no answer: {c().kind === "text" ? "the default stands" : c().default}</span></Show>
        </div>
      </div>
    </Frame>
  );
}

export function PermissionCard(props: { ask: PermissionAsk }) {
  const [done, setDone] = createSignal<string | null>(null);
  const decide = async (d: "allow" | "always" | "deny") => {
    setDone(d === "deny" ? "You denied it." : d === "always" ? "Allowed, and remembered for this agent." : "Allowed once.");
    setTimeout(() => setPermissions((l) => l.filter((a) => a.id !== props.ask.id)), FOLD_MS);
    await api.answerPermission(props.ask.id, d).catch(fail);
  };
  return (
    <Frame agent={props.ask.agent} verb="wants to run a command" meta={`${ago(props.ask.at)} · ${props.ask.tool}`} done={done()} chip={<span class="inline-flex items-center gap-1.5 text-xs text-muted-foreground"><Icon icon={ic.shield} size={13} />asks first</span>}>
      <div class="flex flex-col gap-3 px-4 pb-4">
        <code class="block rounded-md bg-muted px-3 py-2 font-mono text-[13px] break-all"><span class="text-muted-foreground">$ </span>{props.ask.summary}</code>
        <div class="flex items-center gap-2">
          <Button size="sm" onClick={() => decide("allow")}>Allow once</Button>
          <Show when={props.ask.can_remember}><Button size="sm" variant="outline" onClick={() => decide("always")}>Always</Button></Show>
          <Button size="sm" variant="ghost" class="text-muted-foreground" onClick={() => decide("deny")}>Deny</Button>
        </div>
      </div>
    </Frame>
  );
}

export function ScreenCard(props: { q: ScreenPrompt }) {
  const [done, setDone] = createSignal<string | null>(null);
  const choose = async (keys: string, label: string) => {
    setDone(`You chose ${label}.`);
    setTimeout(() => setScreenPrompts((l) => l.filter((x) => x.pane !== props.q.pane)), FOLD_MS);
    await api.answerScreen(props.q.pane, keys).catch(fail);
  };
  return (
    <Frame agent={props.q.agent} verb="asks in its terminal" meta="read from the screen" done={done()}
      chip={<button class="inline-flex items-center gap-1.5 text-xs text-muted-foreground transition-colors hover:text-foreground" onClick={() => setDrawerPane(props.q.pane)}><Icon icon={ic.terminal} size={13} />Open</button>}>
      <div class="flex flex-col gap-3 px-4 pb-4">
        <p class="text-sm leading-snug font-medium">{props.q.prompt!.question}</p>
        <div class="flex flex-wrap gap-2">
          <For each={props.q.prompt!.choices}>{(c, i) => <Button size="sm" variant={i() === 0 ? "default" : "outline"} onClick={() => choose(c.keys, c.label)}>{c.label}</Button>}</For>
        </div>
      </div>
    </Frame>
  );
}

/** The project's own config can run things: serious findings hold every launch until read. */
export function TrustCard() {
  const [report, { mutate }] = createResource(() => api.trustReport().catch(() => null));
  const serious = (): TrustFinding[] => (report()?.findings ?? []).filter((f) => f.severity === "high" || f.severity === "critical");
  const [done, setDone] = createSignal(false);
  const accept = async () => {
    try {
      mutate(await api.trustAccept());
      setDone(true);
      toast({ title: "Accepted", body: "Asked again if these files change.", tone: "ok" });
    } catch (e) {
      toast({ title: "Not accepted", body: String(e), tone: "danger" });
    }
  };
  return (
    <Show when={serious().length || done()}>
      <Card class="m-rise gap-0 overflow-hidden rounded-xl py-0 shadow-none">
        <header class="flex items-center gap-2.5 border-b px-4 py-3">
          <span class="grid size-7 place-items-center rounded-lg bg-muted"><Icon icon={done() ? ic.sealCheck : ic.shield} size={16} /></span>
          <div class="flex-1 leading-tight"><p class="text-sm font-medium">Project config</p><p class="text-xs text-muted-foreground">{done() ? "Accepted" : `${serious().length} serious finding${serious().length === 1 ? "" : "s"}`}</p></div>
          <Show when={!done() && report()?.blocked}><span class="inline-flex items-center gap-1.5 text-xs"><span class="m-ring size-1.5 rounded-full bg-signal" />Blocks launches</span></Show>
        </header>
        <Show when={!done()}>
          <ul class="flex flex-col divide-y">
            <For each={serious()}>
              {(f) => (
                <li class="flex gap-3 px-4 py-3">
                  <span class={cx("mt-0.5 inline-flex h-5 shrink-0 items-center rounded-full px-2 text-[11px] font-medium", f.severity === "critical" || f.severity === "high" ? "bg-foreground text-background" : "bg-muted")}>{f.severity}</span>
                  <div class="min-w-0 flex-1 text-sm leading-snug"><p class="font-mono text-xs break-all text-muted-foreground">{f.file}</p><p>{f.message}</p></div>
                </li>
              )}
            </For>
          </ul>
          <Show when={report()?.blocked}>
            <footer class="flex items-center gap-3 border-t px-4 py-3 text-sm"><span class="text-muted-foreground">Only if you wrote these files, or have read them.</span><Button size="sm" class="ml-auto" onClick={accept}>I have read this, accept</Button></footer>
          </Show>
        </Show>
      </Card>
    </Show>
  );
}

/** Every pending decision of the active project, newest first, with the trust scan on top. */
export default function NeedsYou() {
  return (
    <>
      <TrustCard />
      <For each={waitingAsks()}>{(a) => <QuestionCard ask={a} />}</For>
      <For each={waitingPermissions()}>{(a) => <PermissionCard ask={a} />}</For>
      <For each={waitingScreens()}>{(q) => <ScreenCard q={q} />}</For>
    </>
  );
}
