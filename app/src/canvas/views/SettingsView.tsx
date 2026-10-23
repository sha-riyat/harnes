// Settings: one view per area, a nav on the left and the area's rows on the right.
// Every control saves itself and a quiet "Saved" appears in the header, so there is
// no Save button anywhere. A model row shows the role as its icon.

import { For, Match, Show, Switch as Sw, createSignal, onCleanup } from "solid-js";
import { Card } from "@/components/ui/card";
import { Switch, SwitchControl, SwitchInput, SwitchThumb } from "@/components/ui/switch";
import { cx } from "@/lib/cva";
import Icon from "../../ui/Icon";
import { ic } from "../icons";
import AgentIcon from "../ui/AgentIcon";
import { settingSections, type Setting } from "./viewData";

const MODELS = ["Opus 5.5", "Sonnet 5.5", "Haiku 5.5"];
const EFFORT = ["low", "medium", "high"];

function Seg(props: { value: string; options: string[]; onChange: (v: string) => void; label: string }) {
  return (
    <div class="flex rounded-full border p-0.5" role="radiogroup" aria-label={props.label}>
      <For each={props.options}>{(o) => <button role="radio" aria-checked={props.value === o} onClick={() => props.onChange(o)} class={cx("h-7 rounded-full px-3 text-xs transition-colors duration-150", props.value === o ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:text-foreground")}>{o}</button>}</For>
    </div>
  );
}

function Row(props: { s: Setting; saved: () => void }) {
  const [on, setOn] = createSignal(props.s.kind === "switch" ? props.s.on : false);
  const [model, setModel] = createSignal(props.s.kind === "model" ? props.s.model : "");
  const [effort, setEffort] = createSignal(props.s.kind === "model" ? props.s.effort : "");
  const [val, setVal] = createSignal(props.s.kind === "choice" ? props.s.value : "");
  const s = props.s;
  return (
    <div class="flex items-center gap-4 px-5 py-4">
      <Show when={s.kind === "model"}><AgentIcon role={(s as Extract<Setting, { kind: "model" }>).role} size="lg" /></Show>
      <div class="min-w-0 flex-1 leading-snug"><p class="text-sm font-medium">{s.title}</p><p class="text-sm text-muted-foreground">{s.hint}</p></div>
      <Sw>
        <Match when={s.kind === "switch"}>
          <Switch checked={on()} onChange={(v) => (setOn(v), props.saved())} aria-label={s.title}><SwitchInput /><SwitchControl><SwitchThumb /></SwitchControl></Switch>
        </Match>
        <Match when={s.kind === "model"}>
          <div class="flex items-center gap-2"><Seg label="Model" value={model()} options={MODELS} onChange={(v) => (setModel(v), props.saved())} /><Seg label="Effort" value={effort()} options={EFFORT} onChange={(v) => (setEffort(v), props.saved())} /></div>
        </Match>
        <Match when={s.kind === "choice"}>
          <div class="flex flex-col items-stretch gap-1.5" role="radiogroup" aria-label={s.title}>
            <For each={(s as Extract<Setting, { kind: "choice" }>).options}>
              {(o) => (
                <button role="radio" aria-checked={val() === o.id} onClick={(_id) => (setVal(o.id), props.saved())} class={cx("flex w-56 items-center gap-3 rounded-lg border px-3 py-2 text-left transition-colors duration-150", val() === o.id ? "border-foreground" : "hover:bg-accent/50")}>
                  <span class={cx("grid size-4 place-items-center rounded-full border", val() === o.id && "border-foreground")}><Show when={val() === o.id}><span class="m-pop size-2 rounded-full bg-foreground" /></Show></span>
                  <span class="leading-tight"><span class="block text-sm">{o.label}</span><span class="block text-xs text-muted-foreground">{o.hint}</span></span>
                </button>
              )}
            </For>
          </div>
        </Match>
      </Sw>
    </div>
  );
}

export default function SettingsView(props: { initial?: string }) {
  const [sel, setSel] = createSignal(props.initial ?? "crew");
  const [stamp, setStamp] = createSignal(false);
  let timer = 0;
  onCleanup(() => clearTimeout(timer));
  const saved = () => {
    setStamp(true);
    clearTimeout(timer);
    timer = window.setTimeout(() => setStamp(false), 1800);
  };
  const section = () => settingSections.find((x) => x.id === sel())!;
  return (
    <div class="flex min-h-0 flex-1 flex-col gap-4 p-5">
      <header class="flex items-end gap-4 px-1">
        <div class="flex-1"><h1 class="text-2xl font-semibold tracking-tight">Settings</h1><p class="text-sm text-muted-foreground">Every change is saved as you make it.</p></div>
        <span class={cx("inline-flex items-center gap-1.5 text-sm text-muted-foreground transition-opacity duration-300", stamp() ? "opacity-100" : "opacity-0")} role="status"><Icon icon={ic.checkBold} size={13} />Saved</span>
      </header>
      <div class="grid min-h-0 flex-1 grid-cols-[220px_minmax(0,1fr)] gap-6">
        <nav class="flex flex-col gap-0.5" aria-label="Settings">
          <For each={settingSections}>
            {(x) => (
              <button class={cx("flex h-9 items-center gap-2.5 rounded-lg px-2.5 text-sm transition-colors", sel() === x.id ? "bg-accent text-foreground" : "text-muted-foreground hover:bg-accent/50 hover:text-foreground")} aria-current={sel() === x.id} onClick={() => setSel(x.id)}>
                <Icon icon={ic[x.icon]} size={16} />{x.label}
              </button>
            )}
          </For>
        </nav>
        <div class="min-h-0 overflow-y-auto pr-1">
          <Show when={sel()} keyed>
            {(_id) => (
              <div class="m-rise flex max-w-3xl flex-col gap-3">
                <div class="px-1"><h2 class="text-base font-semibold">{section().label}</h2><p class="text-sm text-muted-foreground">{section().note}</p></div>
                <Card class="gap-0 divide-y rounded-xl py-0 shadow-none">
                  <For each={section().rows}>{(r) => <Row s={r} saved={saved} />}</For>
                </Card>
              </div>
            )}
          </Show>
        </div>
      </div>
    </div>
  );
}
