// The design canvas host: camera, gestures, saved layout. It knows nothing about
// the previews; adding one is a registry edit (registry.ts), never a change here.

import { For, createSignal, onCleanup, onMount } from "solid-js";
import { Dynamic } from "solid-js/web";
import { applyTheme, theme } from "../ui/state";
import { clamp, fit, safeCamera, zoomAt, type Box, type Camera } from "./camera";
import Frame, { BAR } from "./Frame";
import Hud from "./Hud";
import { items, LAYOUT_REV } from "./registry";
import { load, save } from "./storage";
import "./tw.css";
import "./canvas.css";

const HOME: Camera = { x: 48, y: 72, z: 1 };
const ZOOM_STEP = 1.25;
const INTERACTIVE = "input,textarea,select,button,a[href],[contenteditable],[role=button],[role=radio]";
const CHROME = ".cv-hud, .cv-theme";

export default function Canvas() {
  const saved = load(new Set(items.map((i) => i.id)), LAYOUT_REV);
  const pos: Record<string, [number, number]> = {};
  for (const it of items) pos[it.id] = saved.pos[it.id] ?? [it.x, it.y];
  let cam: Camera = saved.camera ?? HOME;

  let root!: HTMLDivElement;
  let layer!: HTMLDivElement;
  const els = new Map<string, HTMLElement>();
  const dirty = new Set<string>();
  const [zoom, setZoom] = createSignal(Math.round(cam.z * 100));
  const mq = matchMedia("(prefers-color-scheme: dark)");
  const [dark, setDark] = createSignal(mq.matches);
  const mode = () => (theme() === "system" ? (dark() ? "dark" : "light") : theme());

  let raf = 0;
  let saveTimer = 0;
  let animTimer = 0;
  let front = 1;
  let space = false;
  let suppress = false;
  let pan: { id: number; sx: number; sy: number; cam: Camera; moved: boolean } | null = null;
  let pinch: { d: number; cx: number; cy: number; cam: Camera } | null = null;

  const paint = () => {
    raf = 0;
    layer.style.transform = `translate(${cam.x}px, ${cam.y}px) scale(${cam.z})`;
    setZoom(Math.round(cam.z * 100));
    for (const id of dirty) {
      const el = els.get(id);
      if (el) el.style.transform = `translate(${pos[id][0]}px, ${pos[id][1]}px)`;
    }
    dirty.clear();
  };
  const schedule = () => {
    raf ||= requestAnimationFrame(paint);
  };
  const persist = () => {
    clearTimeout(saveTimer);
    saveTimer = window.setTimeout(flush, 400);
  };
  const flush = () => {
    clearTimeout(saveTimer);
    save({ camera: cam, pos }, LAYOUT_REV);
  };
  const setCam = (next: Camera, animate = false) => {
    cam = safeCamera(next);
    if (animate) {
      layer.classList.add("cv-anim");
      clearTimeout(animTimer);
      animTimer = window.setTimeout(() => layer.classList.remove("cv-anim"), 220);
    }
    schedule();
    persist();
  };
  const size = () => ({ w: root.clientWidth, h: root.clientHeight });
  const boxes = (): Box[] => items.map((i) => ({ x: pos[i.id][0], y: pos[i.id][1] - BAR, w: i.w, h: i.h + BAR }));
  const fitAll = () => setCam(fit(boxes(), size().w, size().h), true);
  // Put every item back where the source places it, then show them all.
  const tidy = () => {
    for (const it of items) {
      pos[it.id] = [it.x, it.y];
      dirty.add(it.id);
    }
    setCam(fit(boxes(), size().w, size().h), true);
  };
  const zoomBy = (dir: 1 | -1) => {
    const { w, h } = size();
    setCam(zoomAt(cam, cam.z * (dir > 0 ? ZOOM_STEP : 1 / ZOOM_STEP), w / 2, h / 2), true);
  };

  // A nested scroller that can still move in the gesture direction keeps the wheel.
  const scrollsInside = (target: EventTarget | null, dx: number, dy: number) => {
    for (let el = target as HTMLElement | null; el && el !== root && el.closest(".cv-body"); el = el.parentElement) {
      const s = getComputedStyle(el);
      const y = /auto|scroll/.test(s.overflowY) && el.scrollHeight > el.clientHeight && (dy < 0 ? el.scrollTop > 0 : dy > 0 && el.scrollTop + el.clientHeight < el.scrollHeight - 1);
      const x = /auto|scroll/.test(s.overflowX) && el.scrollWidth > el.clientWidth && (dx < 0 ? el.scrollLeft > 0 : dx > 0 && el.scrollLeft + el.clientWidth < el.scrollWidth - 1);
      if (y || x) return true;
    }
    return false;
  };

  const onWheel = (e: WheelEvent) => {
    if ((e.target as Element).closest(CHROME)) return;
    const unit = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? root.clientHeight : 1;
    let dx = e.deltaX * unit;
    let dy = e.deltaY * unit;
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      const r = root.getBoundingClientRect();
      setCam(zoomAt(cam, cam.z * Math.exp(-dy * 0.0022), e.clientX - r.left, e.clientY - r.top));
      return;
    }
    if (e.shiftKey && !dx) [dx, dy] = [dy, 0];
    if (scrollsInside(e.target, dx, dy)) return;
    e.preventDefault();
    setCam({ ...cam, x: cam.x - dx, y: cam.y - dy });
  };

  const onPointerDown = (e: PointerEvent) => {
    if (pinch || (e.target as Element).closest(CHROME)) return;
    const explicit = e.button === 1 || (e.button === 0 && space);
    const empty = e.button === 0 && !(e.target as Element).closest(".cv-item");
    if (!explicit && !empty) return;
    if (explicit) e.stopPropagation(); // pan wins over item move, name copy and the preview
    pan = { id: e.pointerId, sx: e.clientX, sy: e.clientY, cam, moved: false };
    try {
      root.setPointerCapture(e.pointerId);
    } catch {
      // The pointer is already gone; the pan still ends on pointerup.
    }
  };
  const onPointerMove = (e: PointerEvent) => {
    if (!pan || e.pointerId !== pan.id) return;
    const dx = e.clientX - pan.sx;
    const dy = e.clientY - pan.sy;
    if (Math.hypot(dx, dy) > 3) pan.moved = true;
    setCam({ ...pan.cam, x: pan.cam.x + dx, y: pan.cam.y + dy });
  };
  const endPan = (e: PointerEvent) => {
    if (!pan || e.pointerId !== pan.id) return;
    if (pan.moved) {
      suppress = true; // the click that follows a pan must not reach a control
      setTimeout(() => (suppress = false), 60);
    }
    pan = null;
  };
  const swallow = (e: Event) => {
    if (suppress) {
      e.stopPropagation();
      e.preventDefault();
    }
  };
  // Space-drag and middle-drag must not start a text selection or autoscroll.
  const onMouseDown = (e: MouseEvent) => {
    if (e.button === 1 || (e.button === 0 && space && !(e.target as Element).closest(CHROME))) e.preventDefault();
  };

  const touchState = (e: TouchEvent) => {
    const [a, b] = [e.touches[0], e.touches[1]];
    const r = root.getBoundingClientRect();
    return { d: Math.hypot(a.clientX - b.clientX, a.clientY - b.clientY), cx: (a.clientX + b.clientX) / 2 - r.left, cy: (a.clientY + b.clientY) / 2 - r.top };
  };
  const onTouchStart = (e: TouchEvent) => {
    if (e.touches.length !== 2) return;
    pan = null;
    pinch = { ...touchState(e), cam };
  };
  const onTouchMove = (e: TouchEvent) => {
    if (!pinch || e.touches.length !== 2) return;
    e.preventDefault();
    const t = touchState(e);
    const z = clamp(pinch.cam.z * (t.d / pinch.d), 0.15, 2);
    const wx = (pinch.cx - pinch.cam.x) / pinch.cam.z;
    const wy = (pinch.cy - pinch.cam.y) / pinch.cam.z;
    setCam({ z, x: t.cx - wx * z, y: t.cy - wy * z });
  };
  const onTouchEnd = (e: TouchEvent) => {
    if (e.touches.length < 2) pinch = null;
  };

  const interactive = () => {
    const a = document.activeElement;
    return !!a && a !== document.body && a !== root && (a.matches(INTERACTIVE) || (a as HTMLElement).isContentEditable);
  };
  const release = () => {
    space = false;
    pan = null;
    pinch = null;
  };
  const onWinKey = (e: KeyboardEvent) => {
    if (e.code === "Space" && !interactive() && !e.repeat) {
      space = e.type === "keydown";
      if (space) e.preventDefault();
    }
    if (e.type === "keyup" && e.code === "Space") space = false;
  };
  const onRootKey = (e: KeyboardEvent) => {
    if (e.target !== root || e.metaKey || e.ctrlKey || e.altKey) return;
    const step = e.shiftKey ? 240 : 60;
    const move: Record<string, [number, number]> = { ArrowLeft: [step, 0], ArrowRight: [-step, 0], ArrowUp: [0, step], ArrowDown: [0, -step] };
    const m = move[e.key];
    if (m) setCam({ ...cam, x: cam.x + m[0], y: cam.y + m[1] }, true);
    else if (e.key === "+" || e.key === "=") zoomBy(1);
    else if (e.key === "-") zoomBy(-1);
    else if (e.key === "0") setCam(HOME, true);
    else if (e.key.toLowerCase() === "f") fitAll();
    else return;
    e.preventDefault();
  };

  // Keyboard focus that lands off screen is brought back by moving the camera.
  const onFocusIn = (e: FocusEvent) => {
    const t = e.target as Element;
    if (!t.closest(".cv-item") || !t.matches(":focus-visible")) return;
    const r = t.getBoundingClientRect();
    const v = root.getBoundingClientRect();
    const dx = r.left < v.left + 24 ? v.left + 24 - r.left : r.right > v.right - 24 ? v.right - 24 - r.right : 0;
    const dy = r.top < v.top + 24 ? v.top + 24 - r.top : r.bottom > v.bottom - 80 ? v.bottom - 80 - r.bottom : 0;
    if (dx || dy) setCam({ ...cam, x: cam.x + dx, y: cam.y + dy }, true);
  };

  const moveTo = (id: string, x: number, y: number) => {
    pos[id] = [clamp(x, -100_000, 100_000), clamp(y, -100_000, 100_000)];
    dirty.add(id);
    schedule();
    persist();
  };
  const raise = (id: string) => {
    const el = els.get(id);
    if (el) el.style.zIndex = String(++front);
  };
  const onHandleDown = (id: string) => (e: PointerEvent) => {
    if (e.button !== 0 || space) return;
    const h = e.currentTarget as HTMLElement;
    try {
      h.setPointerCapture(e.pointerId);
    } catch {
      // Same as above: the drag ends on pointerup or cancel.
    }
    raise(id);
    const [px, py] = pos[id];
    const move = (m: PointerEvent) => m.pointerId === e.pointerId && moveTo(id, px + (m.clientX - e.clientX) / cam.z, py + (m.clientY - e.clientY) / cam.z);
    const end = () => {
      h.removeEventListener("pointermove", move);
      for (const t of ["pointerup", "pointercancel", "lostpointercapture"] as const) h.removeEventListener(t, end);
      flush();
    };
    h.addEventListener("pointermove", move);
    for (const t of ["pointerup", "pointercancel", "lostpointercapture"] as const) h.addEventListener(t, end);
  };
  const onHandleKey = (id: string) => (e: KeyboardEvent) => {
    const step = e.shiftKey ? 50 : 10;
    const d: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] };
    if (!d[e.key]) return;
    e.preventDefault();
    e.stopPropagation();
    raise(id);
    moveTo(id, pos[id][0] + d[e.key][0], pos[id][1] + d[e.key][1]);
  };

  onMount(() => {
    applyTheme();
    const html = document.documentElement;
    const prior = [html.style.overscrollBehaviorX, document.body.style.overscrollBehaviorX];
    html.style.overscrollBehaviorX = document.body.style.overscrollBehaviorX = "none";
    const onMq = () => setDark(mq.matches);
    const onHide = () => document.hidden && release();
    const wins: [string, EventListener][] = [["keydown", onWinKey as EventListener], ["keyup", onWinKey as EventListener], ["blur", release], ["pagehide", flush]];
    wins.forEach(([t, f]) => window.addEventListener(t, f));
    document.addEventListener("visibilitychange", onHide);
    mq.addEventListener("change", onMq);
    // A new item shows up in a saved layout: reveal the whole set instead of hiding it.
    const fresh = items.some((i) => !(i.id in saved.pos));
    if (!saved.camera || fresh) cam = fit(boxes(), size().w, size().h);
    for (const it of items) dirty.add(it.id);
    paint();
    onCleanup(() => {
      wins.forEach(([t, f]) => window.removeEventListener(t, f));
      document.removeEventListener("visibilitychange", onHide);
      mq.removeEventListener("change", onMq);
      cancelAnimationFrame(raf);
      clearTimeout(animTimer);
      flush();
      [html.style.overscrollBehaviorX, document.body.style.overscrollBehaviorX] = prior;
    });
  });

  return (
    <div
      class="cv-root"
      data-mode={mode()}
      tabindex="0"
      aria-label="Design canvas. Arrow keys pan, plus and minus zoom, F fits all, 0 resets."
      ref={(el) => {
        root = el;
        const on = (t: string, f: (e: any) => void, o?: AddEventListenerOptions) => el.addEventListener(t, f, o);
        on("pointerdown", onPointerDown, { capture: true });
        on("pointermove", onPointerMove);
        on("pointerup", endPan);
        on("pointercancel", endPan);
        on("lostpointercapture", endPan);
        on("mousedown", onMouseDown, { capture: true });
        on("click", swallow, { capture: true });
        on("auxclick", swallow, { capture: true });
        on("keydown", onRootKey);
        on("focusin", onFocusIn);
        on("scroll", () => ((el.scrollTop = 0), (el.scrollLeft = 0)));
        on("wheel", onWheel, { passive: false });
        on("touchstart", onTouchStart, { passive: true });
        on("touchmove", onTouchMove, { passive: false });
        on("touchend", onTouchEnd);
        on("touchcancel", onTouchEnd);
      }}
    >
      <div class="cv-layer" ref={layer}>
        <For each={items}>
          {(it) => (
            <Frame id={it.id} name={it.name} w={it.w} h={it.h} ref={(el) => els.set(it.id, el)} onHandleDown={onHandleDown(it.id)} onHandleKey={onHandleKey(it.id)}>
              <Dynamic component={it.Component} />
            </Frame>
          )}
        </For>
      </div>
      <Hud zoom={zoom} onZoom={zoomBy} onFit={fitAll} onReset={() => setCam(HOME, true)} onTidy={tidy} />
    </div>
  );
}
