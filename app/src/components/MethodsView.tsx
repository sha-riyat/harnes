// OS packs: the methods the crew applies. Each role loads its packs; the
// rest are reached on demand (os_route). Read-only here, plus import.

import { open } from "@tauri-apps/plugin-dialog";
import { For, Show, createMemo, createResource, createSignal } from "solid-js";
import { api, type OsPack } from "../api";
import AsciiField from "../ui/ascii/AsciiField";
import Icon from "../ui/Icon";
import { icons as D } from "../ui/icons";
import { toast } from "../ui/state";

const ROLES: [string, string][] = [
  ["orchestrator", "orchestrator"],
  ["planner", "planner"],
  ["worker", "workers"],
  ["reviewer", "reviewer"],
  ["teacher", "teacher"],
  ["scout", "scouts"],
];

const short = (id: string) => id.replace(/-os$/, "");

async function importPacks(dir: boolean, refetch: () => void) {
  const picked = await open(dir ? { directory: true, title: "Dossier de packs OS" } : { multiple: false, title: "Pack OS (.zip)", filters: [{ name: "zip", extensions: ["zip"] }] });
  if (!picked || Array.isArray(picked)) return;
  try {
    const ids = await api.osImport(picked);
    toast({ title: `${ids.length} pack(s) importé(s)`, body: "Les projets ouverts les voient déjà.", tone: "ok" });
    refetch();
  } catch (e) {
    toast({ title: "Import impossible", body: String(e), tone: "danger" });
  }
}

function Detail(props: { id: string; back: () => void }) {
  const [d] = createResource(() => props.id, (id) => api.osPack(id).catch(() => null));
  const [skill, setSkill] = createSignal<string | null>(null);
  const [skillText] = createResource(skill, (s) => api.osRead(props.id, `skills/${s}/SKILL.md`).catch(() => ""));
  return (
    <div class="os-detail">
      <button class="btn ghost" onClick={props.back}>
        <Icon icon={D.arrowLeft} size={14} /> méthodes
      </button>
      <Show when={d()}>
        {(x) => (
          <>
            <h2>{short(x().pack.id)}</h2>
            <p class="os-purpose">{x().pack.purpose}</p>
            <p class="label">
              {x().pack.roles.length ? `chargé par : ${x().pack.roles.join(", ")}` : "à la demande · os_route"} · v{x().pack.version}
            </p>
            <h3 class="label">skills</h3>
            <div class="chips">
              <For each={x().pack.skills}>
                {(s) => (
                  <button class="chip" classList={{ on: skill() === s }} onClick={() => setSkill(skill() === s ? null : s)}>
                    {s}
                  </button>
                )}
              </For>
            </div>
            <Show when={skill() && skillText()}>
              <pre class="os-text">{skillText()}</pre>
            </Show>
            <h3 class="label">oracle · non-negotiable</h3>
            <pre class="os-text">{x().oracle}</pre>
            <h3 class="label">doctrine</h3>
            <pre class="os-text">{x().os}</pre>
          </>
        )}
      </Show>
    </div>
  );
}

export default function MethodsView() {
  const [lib, { refetch }] = createResource(() => api.osList().catch(() => ({ dir: "", packs: [] as OsPack[], offline: true })));
  const [current, setCurrent] = createSignal<string | null>(null);
  const packs = () => lib()?.packs ?? [];
  const onDemand = createMemo(() => packs().filter((p) => !p.roles.length));

  const Pack = (p: { pack: OsPack }) => (
    <button class="os-pack" onClick={() => setCurrent(p.pack.id)} title={p.pack.purpose}>
      <span class="os-id">{short(p.pack.id)}</span>
      <span class="dim">{p.pack.purpose}</span>
    </button>
  );

  return (
    <section class="methods">
      <Show when={!current()} fallback={<Detail id={current()!} back={() => setCurrent(null)} />}>
        <header class="chat-bar">
          <span class="label">{packs().length ? `${packs().length} OS packs · bundled · each loaded by the role that needs it` : (lib() as { offline?: boolean })?.offline ? "backend offline · open the harnes app" : "no packs"}</span>
          <span class="spacer" />
          <button class="btn ghost" onClick={() => importPacks(true, refetch)} title="Ajouter d'autres packs">
            <Icon icon={D.plus} size={14} /> Import
          </button>
        </header>

        <Show
          when={packs().length}
          fallback={
            <div class="chat-empty">
              <AsciiField height={180} seed={11} />
              <p class="dim">
                Un pack OS est une méthode complète pour agents : doctrine, oracle, skills, routage. Importe un dossier de zips ou un pack : chaque
                agent de l'équipage chargera ceux de son rôle.
              </p>
              <div class="att-actions">
                <button class="btn solid" onClick={() => importPacks(true, refetch)}>Importer un dossier</button>
                <button class="btn" onClick={() => importPacks(false, refetch)}>Importer un zip</button>
              </div>
            </div>
          }
        >
          <div class="os-roles">
            <For each={ROLES}>
              {([role, label]) => (
                <Show when={packs().some((p) => p.roles.includes(role))}>
                  <div class="os-role">
                    <span class="label">{label}</span>
                    <div class="os-list">
                      <For each={packs().filter((p) => p.roles.includes(role))}>{(p) => <Pack pack={p} />}</For>
                    </div>
                  </div>
                </Show>
              )}
            </For>
            <Show when={onDemand().length}>
              <div class="os-role">
                <span class="label">on demand</span>
                <div class="os-list">
                  <For each={onDemand()}>{(p) => <Pack pack={p} />}</For>
                </div>
              </div>
            </Show>
          </div>
          <p class="label faint os-dir">{lib()?.dir}</p>
        </Show>
      </Show>
    </section>
  );
}
