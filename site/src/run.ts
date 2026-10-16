// The hero's live fragment: a crew run played out the way the app shows it.
// The task you type is the one the crew works on; the permission card waits
// for your answer, like the real one does. Without script the page shows the
// finished run, written in the HTML.

import { decode, mark, thinking, wire } from "../../app/src/ui/ascii/pieces";
import { still } from "./field";

const AGENTS = ["orchestrator", "planner", "teacher", "worker-L1", "worker-L2", "reviewer"] as const;
type Agent = (typeof AGENTS)[number];
const STEPS = ["plan", "bar", "work", "review", "gate", "ready"];

interface Beat {
  at?: Agent;
  state?: string;
  step?: number;
  tool?: string;
  text?: string;
  ask?: boolean;
  wait?: number;
}

const beats = (task: string): Beat[] => [
  { at: "orchestrator", state: "working", tool: "crew_start", text: task, step: 0 },
  { at: "planner", state: "working", tool: "os_brief", text: "project · stepper" },
  { at: "planner", state: "done", tool: "plan.json", text: "2 lots, 1 wave, no shared files", step: 1 },
  { at: "teacher", state: "working", tool: "bar.json", text: "4 criteria, each with its evidence" },
  { at: "teacher", state: "idle", step: 2 },
  { at: "worker-L1", state: "working", tool: "thread_read", text: "lot-1-L1" },
  { at: "worker-L2", state: "working", tool: "thread_read", text: "lot-1-L2" },
  { at: "worker-L1", state: "blocked", ask: true },
  { at: "worker-L1", state: "working", tool: "e2e_observe", text: "#n3 button \"Send\"" },
  { at: "worker-L2", state: "working", tool: "ctx_execute", text: "npm test · 12 passed" },
  { at: "worker-L1", state: "done", tool: "e2e_save", text: "reports/L1-r0.json" },
  { at: "worker-L2", state: "done", tool: "reports", text: "reports/L2-r0.json", step: 3 },
  { at: "reviewer", state: "working", tool: "review-1.json", text: "PASS" },
  { at: "reviewer", state: "done", step: 4 },
  { at: "teacher", state: "working", tool: "gate-1.json", text: "bar met", step: 5 },
  { at: "teacher", state: "done" },
  { at: "orchestrator", state: "done", wait: 0 },
];

export function run(root: HTMLElement) {
  const $ = <T extends Element>(s: string) => root.querySelector(s) as T;
  const input = $<HTMLInputElement>("[data-task]");
  const go = $<HTMLButtonElement>("[data-go]");
  const log = $<HTMLOListElement>("[data-log]");
  const card = $<HTMLElement>("[data-ask]");
  const ready = $<HTMLElement>("[data-ready]");
  const track = $<HTMLElement>("[data-track]");
  const think = $<HTMLElement>("[data-think]");
  const marks = new Map(AGENTS.map((a) => [a, root.querySelector<HTMLElement>(`[data-agent="${a}"]`)!]));
  const state = new Map<Agent, string>(AGENTS.map((a) => [a, "idle"]));
  let step = -1, t0 = performance.now(), stepAt = 0, token = 0;

  // One clock draws every living piece of the fragment.
  const draw = (ms: number) => {
    const t = (ms - t0) / 1000;
    marks.forEach((el, a) => {
      el.querySelector(".mk")!.textContent = mark(state.get(a)!, AGENTS.indexOf(a))(t) || " ";
      el.dataset.state = state.get(a);
    });
    track.textContent = STEPS.map((s, i) => (i < step ? s : i === step ? decode(s)(t - stepAt) : s)).reduce((acc, s, i) =>
      i ? `${acc} ${wire(3, i <= step ? "done" : i === step + 1 ? "now" : "todo")(t)} ${s}` : s, "");
    think.textContent = state.get("orchestrator") === "working" ? thinking(16)(t) : "";
    if (!still) requestAnimationFrame(draw);
  };

  const line = (b: Beat) => {
    const li = document.createElement("li");
    li.innerHTML = `<span class="who"></span><span class="tool"></span><span class="what"></span>`;
    li.children[0].textContent = b.at!;
    li.children[1].textContent = b.tool!;
    li.children[2].textContent = b.text ?? "";
    log.append(li);
    log.scrollTop = log.scrollHeight;
  };

  const answer = () =>
    new Promise<void>((done) => {
      card.hidden = false;
      card.querySelectorAll("button").forEach((btn) => (btn.onclick = () => {
        const allowed = btn.dataset.allow !== undefined;
        card.hidden = true;
        line({ at: "worker-L1", tool: allowed ? "Bash" : "denied", text: allowed ? "npm install nodemailer" : "uses the mailer already in the repo" });
        done();
      }));
    });

  const play = async () => {
    const me = ++token;
    const task = input.value.trim() || input.placeholder;
    log.replaceChildren();
    ready.hidden = true;
    card.hidden = true;
    step = -1;
    AGENTS.forEach((a) => state.set(a, "idle"));
    for (const b of beats(task)) {
      if (me !== token) return;
      if (b.at && b.state) state.set(b.at, b.state);
      if (b.step !== undefined) { step = b.step; stepAt = (performance.now() - t0) / 1000; }
      if (b.ask) await answer();
      else if (b.tool) line(b);
      await new Promise((r) => setTimeout(r, still ? 0 : b.wait ?? 900));
    }
    ready.querySelector("[data-task-echo]")!.textContent = task;
    ready.hidden = false;
  };

  go.addEventListener("click", play);
  input.addEventListener("keydown", (e) => e.key === "Enter" && play());
  requestAnimationFrame(draw);
  // Start on its own once the fragment is seen, so the page performs unasked.
  new IntersectionObserver(([e], io) => { if (e.isIntersecting) { io.disconnect(); play(); } }, { threshold: 0.5 }).observe(root);
}
