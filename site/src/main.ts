// The landing page's behaviour: living fields, the crew run, the showcase
// row, the role tabs, the annotated plate, copy buttons and entrances.

import { flow, wire } from "../../app/src/ui/ascii/pieces";
import { field, still } from "./field";
import { run } from "./run";

document.documentElement.classList.add("js");
const all = <T extends Element>(s: string, r: ParentNode = document) => [...r.querySelectorAll<T>(s)];

// Fields: the canvas says which scenes, seed and size it wants.
for (const c of all<HTMLCanvasElement>("canvas[data-field]")) {
  const d = c.dataset;
  field(c, { seed: +(d.seed ?? 3), cell: +(d.cell ?? 14), density: +(d.density ?? 0.8), ramp: d.field === "ramp", scenes: d.scenes?.split(",").map(Number) });
}

const sim = document.querySelector<HTMLElement>("[data-run]");
if (sim) run(sim);

// Showcase: the centred panel is lit and names itself beside the row.
const row = document.querySelector<HTMLElement>("[data-row]");
if (row) {
  const name = document.querySelector("[data-panel-name]")!;
  const text = document.querySelector("[data-panel-text]")!;
  const panels = all<HTMLElement>("[data-panel]", row);
  const light = (p: HTMLElement) => {
    panels.forEach((x) => x.classList.toggle("lit", x === p));
    name.textContent = p.dataset.name!;
    text.textContent = p.dataset.text!;
  };
  const io = new IntersectionObserver((es) => es.forEach((e) => e.isIntersecting && light(e.target as HTMLElement)), { root: row, threshold: 0.6 });
  panels.forEach((p) => {
    io.observe(p);
    p.addEventListener("click", () => p.scrollIntoView({ behavior: "smooth", inline: "center", block: "nearest" }));
  });
  all<HTMLButtonElement>("[data-step]").forEach((b) =>
    b.addEventListener("click", () => {
      const i = panels.findIndex((p) => p.classList.contains("lit"));
      panels[(i + +b.dataset.step! + panels.length) % panels.length].scrollIntoView({ behavior: "smooth", inline: "center", block: "nearest" });
    }));
  light(panels[0]);
}

// Stops of a run: one at a time. The road plays itself until you pick a stop:
// stops behind are drawn solid, the wire into the current one flows.
for (const tabs of all<HTMLElement>("[data-tabs]")) {
  const buttons = all<HTMLButtonElement>("[role=tab]", tabs);
  let at = 0, auto = tabs.hasAttribute("data-cycle"), stepAt = 0;
  const show = (b: HTMLButtonElement) => {
    at = buttons.indexOf(b);
    stepAt = performance.now();
    buttons.forEach((x, i) => { x.setAttribute("aria-selected", String(x === b)); x.dataset.past = String(i < at); });
    all<HTMLElement>("[role=tabpanel]").forEach((p) => (p.hidden = p.id !== b.getAttribute("aria-controls")));
  };
  const pick = (b: HTMLButtonElement) => { auto = false; show(b); };
  buttons.forEach((b, i) => {
    b.addEventListener("click", () => pick(b));
    b.addEventListener("keydown", (e) => {
      const d = e.key === "ArrowDown" || e.key === "ArrowRight" ? 1 : e.key === "ArrowUp" || e.key === "ArrowLeft" ? -1 : 0;
      if (d) { e.preventDefault(); const n = buttons[(i + d + buttons.length) % buttons.length]; n.focus(); pick(n); }
    });
  });
  show(buttons[0]);
  if (!tabs.hasAttribute("data-cycle")) continue;
  const wires = all<HTMLElement>("[data-wire]", tabs);
  const back = document.querySelector<HTMLElement>("[data-back]");
  let seen = false;
  new IntersectionObserver(([e]) => (seen = e.isIntersecting)).observe(tabs);
  const tick = (ms: number) => {
    const t = ms / 1000;
    wires.forEach((w, i) => (w.textContent = i === 0 ? "" : i < at ? "-".repeat(6) : i === at ? wire(6, "now")(t) : "·".repeat(6)));
    // The way back runs right to left, from review and gate to work.
    if (back) back.textContent = "◄" + [...flow(16, 1e9, 1.8)(t)].reverse().join("").replace(/>/g, "");
    if (auto && seen && ms - stepAt > 2600) show(buttons[(at + 1) % buttons.length]);
    if (!still) requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}

// The annotated plate: a note under the pointer keeps its part lit.
const plate = document.querySelector<HTMLElement>("[data-plate]");
if (plate) {
  // Each note sits level with the part it explains, below the note above it.
  const align = () => all<HTMLElement>(".notes", plate).forEach((col) => {
    let floor = 0;
    for (const n of all<HTMLElement>("[data-note]", col)) {
      const part = plate.querySelector<HTMLElement>(`[data-part="${n.dataset.note}"]`)!;
      const top = Math.max(part.getBoundingClientRect().top - plate.getBoundingClientRect().top, floor);
      n.style.setProperty("--top", `${top}px`);
      floor = top + n.offsetHeight + 20;
    }
  });
  new ResizeObserver(align).observe(plate);
  for (const n of all<HTMLElement>("[data-note]")) {
    const on = () => { plate.dataset.focus = n.dataset.note; };
    const off = () => { delete plate.dataset.focus; };
    n.addEventListener("pointerenter", on);
    n.addEventListener("pointerleave", off);
    n.addEventListener("focusin", on);
    n.addEventListener("focusout", off);
  }
}

// Copy buttons on code.
for (const b of all<HTMLButtonElement>("[data-copy]"))
  b.addEventListener("click", async () => {
    await navigator.clipboard.writeText(document.getElementById(b.dataset.copy!)!.textContent!.trim());
    b.textContent = "Copied";
    setTimeout(() => (b.textContent = "Copy"), 1400);
  });

// Entrances: each section's parts arrive in reading order, once.
const io = new IntersectionObserver((es) => es.forEach((e) => {
  if (!e.isIntersecting) return;
  all<HTMLElement>("[data-in]", e.target).forEach((el, i) => (el.style.transitionDelay = `${i * 75}ms`));
  e.target.classList.add("in");
  io.unobserve(e.target);
}), { threshold: 0.15 });
all("[data-reveal]").forEach((s) => io.observe(s));
