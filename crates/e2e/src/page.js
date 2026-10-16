// Injected into every page. Computes ARIA-ish roles and accessible names,
// produces a compact snapshot, and resolves locators. Kept dependency-free.
(() => {
  if (window.__h) return;
  const TAGS = {
    BUTTON: "button", TEXTAREA: "textbox", SELECT: "combobox", IMG: "img", NAV: "navigation",
    MAIN: "main", HEADER: "banner", FOOTER: "contentinfo", FORM: "form", TABLE: "table",
    TR: "row", TD: "cell", TH: "columnheader", UL: "list", OL: "list", LI: "listitem",
    DIALOG: "dialog", ASIDE: "complementary", SUMMARY: "button", PROGRESS: "progressbar",
    OPTION: "option", ARTICLE: "article", P: null, SECTION: null,
  };
  const INPUTS = {
    checkbox: "checkbox", radio: "radio", button: "button", submit: "button", reset: "button",
    image: "button", range: "slider", search: "searchbox", number: "spinbutton", hidden: null,
  };
  const norm = (s) => String(s ?? "").replace(/\s+/g, " ").trim();

  function role(el) {
    const r = el.getAttribute("role");
    if (r) return r.split(" ")[0];
    const t = el.tagName;
    if (t === "A") return el.hasAttribute("href") ? "link" : null;
    if (/^H[1-6]$/.test(t)) return "heading";
    if (t === "INPUT") {
      const ty = (el.getAttribute("type") || "text").toLowerCase();
      return ty in INPUTS ? INPUTS[ty] : "textbox";
    }
    return TAGS[t] ?? null;
  }

  function labelText(el) {
    if (el.id) {
      const l = document.querySelector(`label[for="${CSS.escape(el.id)}"]`);
      if (l) return norm(l.innerText);
    }
    const wrap = el.closest("label");
    return wrap ? norm(wrap.innerText) : "";
  }

  function name(el) {
    const aria = el.getAttribute("aria-label");
    if (aria) return norm(aria);
    const by = el.getAttribute("aria-labelledby");
    if (by) return norm(by.split(/\s+/).map((id) => document.getElementById(id)?.innerText ?? "").join(" "));
    const t = el.tagName;
    if (t === "INPUT" || t === "TEXTAREA" || t === "SELECT") {
      if (["submit", "button", "reset"].includes(el.type)) return norm(el.value);
      return labelText(el) || norm(el.getAttribute("placeholder")) || norm(el.getAttribute("title"));
    }
    if (t === "IMG") return norm(el.getAttribute("alt") || el.getAttribute("title"));
    const text = norm(el.innerText);
    return (text.length > 120 ? text.slice(0, 117) + "…" : text) || norm(el.getAttribute("title"));
  }

  function visible(el) {
    const r = el.getBoundingClientRect();
    if (r.width === 0 || r.height === 0) return false;
    const s = getComputedStyle(el);
    return s.visibility !== "hidden" && s.display !== "none" && s.opacity !== "0";
  }

  let next = 1;
  const hid = (el) => el.dataset.hid || (el.dataset.hid = "n" + next++);

  function snapshot() {
    const nodes = [];
    for (const el of document.querySelectorAll("body *")) {
      const r = role(el);
      if (!(r || el.dataset.testid) || !visible(el)) continue;
      const states = [];
      if (el.disabled) states.push("disabled");
      if (el.checked) states.push("checked");
      if (document.activeElement === el) states.push("focused");
      if (el.getAttribute("aria-expanded") === "true") states.push("expanded");
      const value = "value" in el && !["BUTTON", "LI", "OPTION", "PROGRESS"].includes(el.tagName) ? String(el.value) : null;
      nodes.push({ id: hid(el), role: r || "generic", name: name(el), value, states, testid: el.dataset.testid || null });
      if (nodes.length >= 400) break;
    }
    return { url: location.href, title: document.title, nodes };
  }

  function matches(el, q) {
    const cmp = (a, b) => (q.exact ? norm(a) === norm(b) : norm(a).toLowerCase().includes(norm(b).toLowerCase()));
    if (q.testid != null && el.dataset.testid !== q.testid) return false;
    if (q.id != null && el.dataset.hid !== q.id) return false;
    if (q.role != null && role(el) !== q.role) return false;
    if (q.name != null && !cmp(name(el), q.name)) return false;
    if (q.label != null && !cmp(labelText(el) || el.getAttribute("aria-label") || "", q.label)) return false;
    if (q.placeholder != null && !cmp(el.getAttribute("placeholder") || "", q.placeholder)) return false;
    if (q.text != null && !cmp(el.innerText || el.textContent || "", q.text)) return false;
    return true;
  }

  function find(q) {
    let all = [...document.querySelectorAll(q.css || "body *")].filter((el) => matches(el, q) && visible(el));
    // A bare text query matches every ancestor too: keep the innermost elements.
    if (q.text != null && q.role == null && q.css == null) all = all.filter((el) => !all.some((o) => o !== el && el.contains(o)));
    return all;
  }

  function query(q) {
    document.querySelectorAll("[data-harnes-target]").forEach((e) => e.removeAttribute("data-harnes-target"));
    const els = find(q);
    const el = els[q.nth || 0];
    if (el) el.setAttribute("data-harnes-target", "1");
    return {
      count: els.length,
      text: el ? norm(el.innerText || el.textContent || "") : null,
      value: el && "value" in el ? String(el.value) : null,
      id: el ? hid(el) : null,
    };
  }

  function markFocused() {
    document.querySelectorAll("[data-harnes-target]").forEach((e) => e.removeAttribute("data-harnes-target"));
    (document.activeElement || document.body).setAttribute("data-harnes-target", "1");
    return true;
  }

  window.__h = { snapshot, query, markFocused };
})();
