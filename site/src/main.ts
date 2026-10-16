// The landing page's behaviour: living fields, the crew run, the showcase
// row, the role tabs, the annotated plate, copy buttons and entrances.

import { field } from "./field";
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

// Role tabs: one role's work at a time.
for (const tabs of all<HTMLElement>("[data-tabs]")) {
  const buttons = all<HTMLButtonElement>("[role=tab]", tabs);
  const show = (b: HTMLButtonElement) => {
    buttons.forEach((x) => x.setAttribute("aria-selected", String(x === b)));
    all<HTMLElement>("[role=tabpanel]").forEach((p) => (p.hidden = p.id !== b.getAttribute("aria-controls")));
  };
  buttons.forEach((b, i) => {
    b.addEventListener("click", () => show(b));
    b.addEventListener("keydown", (e) => {
      const d = e.key === "ArrowDown" || e.key === "ArrowRight" ? 1 : e.key === "ArrowUp" || e.key === "ArrowLeft" ? -1 : 0;
      if (d) { e.preventDefault(); const n = buttons[(i + d + buttons.length) % buttons.length]; n.focus(); show(n); }
    });
  });
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
