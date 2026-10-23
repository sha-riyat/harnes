// The landing page's behaviour: the interface tabs, copy buttons and entrances.
// The page reads fine without any of it.

document.documentElement.classList.add("js");
const all = <T extends Element>(s: string, r: ParentNode = document) => [...r.querySelectorAll<T>(s)];

// Interface tabs: one screenshot at a time, arrows move between them.
for (const tabs of all<HTMLElement>("[data-tabs]")) {
  const buttons = all<HTMLButtonElement>("[role=tab]", tabs);
  const show = (b: HTMLButtonElement) => {
    buttons.forEach((x) => x.setAttribute("aria-selected", String(x === b)));
    all<HTMLElement>("[role=tabpanel]").forEach((p) => (p.hidden = p.id !== b.getAttribute("aria-controls")));
  };
  buttons.forEach((b, i) => {
    b.addEventListener("click", () => show(b));
    b.addEventListener("keydown", (e) => {
      const d = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
      if (!d) return;
      e.preventDefault();
      const n = buttons[(i + d + buttons.length) % buttons.length];
      n.focus();
      show(n);
    });
  });
}

// Copy buttons: the command, then a short "Copied".
for (const b of all<HTMLButtonElement>("[data-copy]")) {
  b.addEventListener("click", async () => {
    const text = document.getElementById(b.dataset.copy!)?.textContent ?? "";
    const label = b.textContent;
    try {
      await navigator.clipboard.writeText(text);
      b.textContent = b.dataset.done ?? "Copied";
      setTimeout(() => (b.textContent = label), 1400);
    } catch {
      // No clipboard access: the text stays selectable.
    }
  });
}

// Entrances: each part rises once as it comes into view.
const io = new IntersectionObserver((es) => es.forEach((e) => e.isIntersecting && (e.target.classList.add("in"), io.unobserve(e.target))), { threshold: 0.15 });
all("[data-in]").forEach((el) => io.observe(el));
