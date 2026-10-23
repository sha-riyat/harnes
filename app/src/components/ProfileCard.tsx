// Who may see this project's code, and what its own config could run.
// The profile decides whether free OpenCode models take part; the default,
// Claude only, sends nothing to a third party. Serious findings in the
// project's config stop every launch until the user has read and accepted them.

import { For, Show, createResource } from "solid-js";
import { api, type Profile } from "../api";
import { toast } from "../ui/state";

const PROFILES: { id: Profile; label: string; hint: string }[] = [
  { id: "claude", label: "claude", hint: "Claude only. Nothing leaves for a third party." },
  { id: "mixed", label: "mixed", hint: "Free models review in deep runs and try small tasks first. Claude checks all of it." },
  { id: "max-eco", label: "max eco", hint: "Also the first try of every worker whose files are not sensitive." },
];

export default function ProfileCard() {
  const [info, { mutate }] = createResource(() => api.profileGet().catch(() => null));
  const [trust, { mutate: setTrust }] = createResource(() => api.trustReport().catch(() => null));

  const pick = async (profile: Profile) => {
    try {
      mutate(await api.profileSet(profile));
    } catch (e) {
      toast({ title: "Profile not saved", body: String(e), tone: "danger" });
    }
  };
  const toggle = async (key: "autoEscalate" | "digest", value: boolean) => {
    const c = info()!.config;
    try {
      mutate(await api.profileSet(c.profile, { [key]: value }));
    } catch (e) {
      toast({ title: "Not saved", body: String(e), tone: "danger" });
    }
  };
  const accept = async () => {
    try {
      setTrust(await api.trustAccept());
      toast({ title: "Accepted", body: "Asked again if these files change.", tone: "ok" });
    } catch (e) {
      toast({ title: "Not accepted", body: String(e), tone: "danger" });
    }
  };
  const serious = () => (trust()?.findings ?? []).filter((f) => f.severity === "high" || f.severity === "critical");

  return (
    <>
      <Show when={serious().length}>
        <section class="week trust">
          <header class="week-head">
            <span class="label">project config</span>
            <Show when={trust()?.blocked}><span class="badge">blocks launches</span></Show>
          </header>
          <ul class="finds">
            <For each={serious()}>
              {(f) => (
                <li>
                  <span class="label">{f.severity} · {f.file}</span>
                  <span>{f.message}</span>
                </li>
              )}
            </For>
          </ul>
          <Show when={trust()?.blocked}>
            <button class="btn sm" onClick={accept} title="Only if you wrote these files, or have read them">I have read this, accept</button>
          </Show>
        </section>
      </Show>

      <Show when={info()}>
        {(i) => (
          <section class="week">
            <header class="week-head">
              <span class="label">models</span>
              <Show when={i().effective !== i().config.profile}><span class="badge">{i().effective.replace("-", " ")}</span></Show>
            </header>
            <div class="seg" role="radiogroup" aria-label="Profile">
              <For each={PROFILES}>
                {(p) => (
                  <button role="radio" aria-checked={i().config.profile === p.id} classList={{ on: i().config.profile === p.id }} title={p.hint} onClick={() => pick(p.id)}>
                    {p.label}
                  </button>
                )}
              </For>
            </div>
            <p class="week-line">{PROFILES.find((p) => p.id === i().config.profile)?.hint}</p>
            <Show when={i().config.profile !== "claude"}>
              <p class="week-line">
                <Show when={i().opencode} fallback={<>OpenCode was not found: nothing is routed, Claude does it all.</>}>
                  <b>{i().usable.length}</b> free model{i().usable.length === 1 ? "" : "s"} ready{i().usable.length ? ": " + i().usable.map((m) => m.replace("opencode/", "")).join(", ") : ", none of the allowed ones is offered right now"}
                </Show>
              </p>
              <label class="check"><input type="checkbox" checked={i().config.auto_escalate} onChange={(e) => toggle("autoEscalate", e.currentTarget.checked)} /> mixed becomes max eco when the week runs ahead</label>
              <label class="check"><input type="checkbox" checked={i().config.digest} onChange={(e) => toggle("digest", e.currentTarget.checked)} /> a free model may summarise large outputs</label>
            </Show>
          </section>
        )}
      </Show>
    </>
  );
}
