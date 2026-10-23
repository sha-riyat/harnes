// Settings: one view per area, a nav on the left and the area's rows on the right.
// Every control saves itself and a quiet "Saved" appears in the header; there is no
// Save button. Only real settings are here: the model profile, the trust scan, what
// may interrupt you, the look, and the projects.

import { For, Show, createResource, createSignal, onCleanup } from "solid-js";
import { api, type Profile } from "../../api";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Switch, SwitchControl, SwitchInput, SwitchThumb } from "@/components/ui/switch";
import { cx } from "@/lib/cva";
import { currentProject, projects } from "../../store";
import Icon from "../../ui/Icon";
import { ic } from "../../ui/ph";
import { addProject, closeProject } from "../../ui/projects";
import { prefs, setPrefs, setSettingsSection, setTheme, settingsSection, sidebarOpen, setSidebarOpen, theme, toast, type Theme } from "../../ui/state";

const PROFILES: { id: Profile; label: string; hint: string }[] = [
  { id: "claude", label: "Claude only", hint: "Nothing leaves for a third party" },
  { id: "mixed", label: "Mixed", hint: "Free models review deep runs and try small tasks first; Claude checks all of it" },
  { id: "max-eco", label: "Max economy", hint: "Also the first try of every worker whose files are not sensitive" },
];
const SECTIONS = [
  { id: "models", label: "Models", icon: ic.sliders, note: "Whether free models may take part. Claude always checks their work." },
  { id: "safety", label: "Safety", icon: ic.lockKey, note: "What harnes checks before the crew starts." },
  { id: "notices", label: "Notifications", icon: ic.bell, note: "What may interrupt you. Questions and permissions always wait in the bell." },
  { id: "general", label: "General", icon: ic.palette, note: "The look of the app." },
  { id: "projects", label: "Projects", icon: ic.folder, note: "Folders harnes may open and work in." },
] as const;

function Toggle(props: { on: boolean; set: (v: boolean) => void; label: string }) {
  return <Switch checked={props.on} onChange={props.set} aria-label={props.label}><SwitchInput /><SwitchControl><SwitchThumb /></SwitchControl></Switch>;
}
function Row(props: { title: string; hint: string; children: any }) {
  return <div class="flex items-center gap-4 px-5 py-4"><div class="min-w-0 flex-1 leading-snug"><p class="text-sm font-medium">{props.title}</p><p class="text-sm text-muted-foreground">{props.hint}</p></div>{props.children}</div>;
}
function Seg<T extends string>(props: { value: T; options: readonly T[]; onChange: (v: T) => void; label: string }) {
  return (
    <div class="flex rounded-full border p-0.5" role="radiogroup" aria-label={props.label}>
      <For each={props.options}>{(o) => <button role="radio" aria-checked={props.value === o} onClick={() => props.onChange(o)} class={cx("h-7 rounded-full px-3 text-xs capitalize transition-colors duration-150", props.value === o ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:text-foreground")}>{o}</button>}</For>
    </div>
  );
}

export default function Settings() {
  const [stamp, setStamp] = createSignal(false);
  let timer = 0;
  onCleanup(() => clearTimeout(timer));
  const saved = () => {
    setStamp(true);
    clearTimeout(timer);
    timer = window.setTimeout(() => setStamp(false), 1800);
  };
  const [info, { mutate }] = createResource(() => api.profileGet().catch(() => null));
  const [trust, { mutate: setTrust }] = createResource(() => api.trustReport().catch(() => null));
  const fail = (title: string) => (e: unknown) => toast({ title, body: String(e), tone: "danger" });

  const pick = async (p: Profile) => { try { mutate(await api.profileSet(p)); saved(); } catch (e) { fail("Profile not saved")(e); } };
  const toggle = async (key: "autoEscalate" | "digest", v: boolean) => { try { mutate(await api.profileSet(info()!.config.profile, { [key]: v })); saved(); } catch (e) { fail("Not saved")(e); } };
  const accept = async () => { try { setTrust(await api.trustAccept()); saved(); } catch (e) { fail("Not accepted")(e); } };
  const reload = async () => { try { await api.guardsReload(); saved(); } catch (e) { fail("Could not reload")(e); } };
  const section = () => SECTIONS.find((s) => s.id === settingsSection()) ?? SECTIONS[0];

  return (
    <div class="flex min-h-0 flex-1 flex-col gap-4 p-5">
      <header class="flex items-end gap-4 px-1">
        <div class="flex-1"><h1 class="text-2xl font-semibold tracking-tight">Settings</h1><p class="text-sm text-muted-foreground">Every change is saved as you make it.</p></div>
        <span class={cx("inline-flex items-center gap-1.5 text-sm text-muted-foreground transition-opacity duration-300", stamp() ? "opacity-100" : "opacity-0")} role="status"><Icon icon={ic.checkBold} size={13} />Saved</span>
      </header>
      <div class="grid min-h-0 flex-1 grid-cols-[220px_minmax(0,1fr)] gap-6">
        <nav class="flex flex-col gap-0.5" aria-label="Settings">
          <For each={SECTIONS}>{(x) => <button class={cx("flex h-9 items-center gap-2.5 rounded-lg px-2.5 text-sm transition-colors", section().id === x.id ? "bg-accent text-foreground" : "text-muted-foreground hover:bg-accent/50 hover:text-foreground")} aria-current={section().id === x.id} onClick={() => setSettingsSection(x.id)}><Icon icon={x.icon} size={16} />{x.label}</button>}</For>
        </nav>
        <div class="min-h-0 overflow-y-auto pr-1">
          <Show when={section().id} keyed>
            {(id) => (
              <div class="m-rise flex max-w-3xl flex-col gap-3">
                <div class="px-1"><h2 class="text-base font-semibold">{section().label}</h2><p class="text-sm text-muted-foreground">{section().note}</p></div>

                <Show when={id === "models"}>
                  <Card class="gap-0 divide-y rounded-xl py-0 shadow-none">
                    <Row title="Profile" hint="Nothing goes to a third party unless you pick another.">
                      <div class="flex flex-col gap-1.5" role="radiogroup" aria-label="Profile">
                        <For each={PROFILES}>{(p) => <button role="radio" aria-checked={info()?.config.profile === p.id} onClick={() => pick(p.id)} class={cx("flex w-72 items-center gap-3 rounded-lg border px-3 py-2 text-left transition-colors duration-150", info()?.config.profile === p.id ? "border-foreground" : "hover:bg-accent/50")}><span class={cx("grid size-4 place-items-center rounded-full border", info()?.config.profile === p.id && "border-foreground")}><Show when={info()?.config.profile === p.id}><span class="m-pop size-2 rounded-full bg-foreground" /></Show></span><span class="leading-tight"><span class="block text-sm">{p.label}</span><span class="block text-xs text-muted-foreground">{p.hint}</span></span></button>}</For>
                      </div>
                    </Row>
                    <Show when={info() && info()!.config.profile !== "claude"}>
                      <Row title="Free models ready" hint={info()!.opencode ? (info()!.usable.length ? info()!.usable.map((m) => m.replace("opencode/", "")).join(", ") : "None of the allowed ones is offered right now.") : "OpenCode was not found: nothing is routed, Claude does it all."}><span class="num text-sm">{info()!.usable.length}</span></Row>
                      <Row title="Escalate when the week runs ahead" hint="Mixed becomes max economy when the week's usage runs ahead of the pace."><Toggle label="Escalate" on={info()!.config.auto_escalate} set={(v) => toggle("autoEscalate", v)} /></Row>
                      <Row title="Let a free model summarise" hint="Large tool outputs may be summarised by a free model before they reach Claude."><Toggle label="Digest" on={info()!.config.digest} set={(v) => toggle("digest", v)} /></Row>
                    </Show>
                  </Card>
                </Show>

                <Show when={id === "safety"}>
                  <Card class="gap-0 divide-y rounded-xl py-0 shadow-none">
                    <Row title="Trust scan" hint={(trust()?.findings.length ? `${trust()!.findings.length} finding${trust()!.findings.length === 1 ? "" : "s"} in this project's config.` : "No finding in this project's config.") + (trust()?.blocked ? " Launches are blocked until you accept." : "")}>
                      <Show when={trust()?.blocked}><Button size="sm" onClick={accept}>I have read this, accept</Button></Show>
                    </Row>
                    <For each={trust()?.findings ?? []}>{(f) => <div class="flex gap-3 px-5 py-3 text-sm"><span class={cx("mt-0.5 inline-flex h-5 shrink-0 items-center rounded-full px-2 text-[11px] font-medium", f.severity === "high" || f.severity === "critical" ? "bg-foreground text-background" : "bg-muted")}>{f.severity}</span><div class="min-w-0"><p class="font-mono text-xs break-all text-muted-foreground">{f.file}</p><p>{f.message}</p></div></div>}</For>
                    <Row title="Guard rules" hint="Read from this project's guards file. Reload after you edit it."><Button size="sm" variant="outline" onClick={reload}>Reload</Button></Row>
                  </Card>
                </Show>

                <Show when={id === "notices"}>
                  <Card class="gap-0 divide-y rounded-xl py-0 shadow-none">
                    <Row title="A run is ready for you" hint="A toast when the summary is written, even if you are in another project."><Toggle label="Ready toast" on={prefs().toastDone} set={(v) => (setPrefs({ ...prefs(), toastDone: v }), saved())} /></Row>
                    <Row title="A run stops" hint="A toast with the reason."><Toggle label="Stopped toast" on={prefs().toastFail} set={(v) => (setPrefs({ ...prefs(), toastFail: v }), saved())} /></Row>
                  </Card>
                </Show>

                <Show when={id === "general"}>
                  <Card class="gap-0 divide-y rounded-xl py-0 shadow-none">
                    <Row title="Theme" hint="System follows your operating system."><Seg<Theme> label="Theme" value={theme()} options={["system", "light", "dark"]} onChange={(t) => (setTheme(t), saved())} /></Row>
                    <Row title="Sidebar" hint="Projects and agents on the left."><Toggle label="Sidebar" on={sidebarOpen()} set={(v) => (setSidebarOpen(v), saved())} /></Row>
                  </Card>
                </Show>

                <Show when={id === "projects"}>
                  <Card class="gap-0 divide-y rounded-xl py-0 shadow-none">
                    <For each={projects()} fallback={<p class="px-5 py-6 text-sm text-muted-foreground">No project open.</p>}>
                      {(p) => <Row title={p.name} hint={p.root}><div class="flex items-center gap-2"><Show when={p.id === currentProject()?.id}><span class="text-xs text-muted-foreground">active</span></Show><Button size="sm" variant="ghost" class="text-muted-foreground" onClick={() => closeProject(p.id, p.name)}>Close</Button></div></Row>}
                    </For>
                    <div class="px-5 py-3"><Button size="sm" variant="outline" onClick={addProject}><Icon icon={ic.plus} size={15} />Add a project</Button></div>
                  </Card>
                </Show>
              </div>
            )}
          </Show>
        </div>
      </div>
    </div>
  );
}
