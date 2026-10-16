import { createEffect, onCleanup, onMount } from "solid-js";
import { api } from "../api";
import { focused, frameOf, setFocused } from "../store";
import { encodeKey, paste } from "./keys";
import { draw, metrics } from "./render";

/** One Ghostty-rendered terminal pane. */
export default function TermView(props: { id: string }) {
  let box!: HTMLDivElement;
  let canvas!: HTMLCanvasElement;
  let lastSize = "";

  const fit = () => {
    if (box.clientWidth < 40 || box.clientHeight < 20) return;
    const m = metrics();
    const cols = Math.max(20, Math.floor((box.clientWidth - 8) / m.w));
    const rows = Math.max(5, Math.floor((box.clientHeight - 8) / m.h));
    const key = `${cols}x${rows}`;
    if (key === lastSize) return;
    lastSize = key;
    const dpr = window.devicePixelRatio || 1;
    api.resize(props.id, cols, rows, Math.round(m.w * dpr), Math.round(m.h * dpr)).catch(() => {});
  };

  onMount(() => {
    const ro = new ResizeObserver(() => requestAnimationFrame(fit));
    ro.observe(box);
    onCleanup(() => ro.disconnect());
    api.refresh(props.id).catch(() => {});
  });

  // Redraw on each new frame, coalesced to one paint per animation frame.
  let pending = false;
  createEffect(() => {
    const f = frameOf(props.id);
    const isFocused = focused() === props.id;
    if (!f || pending) return;
    pending = true;
    requestAnimationFrame(() => {
      pending = false;
      const latest = frameOf(props.id);
      if (latest) draw(canvas, latest, isFocused);
    });
  });

  createEffect(() => {
    if (focused() === props.id) box.focus({ preventScroll: true });
  });

  const send = (data: string) => api.write(props.id, data).catch(() => {});

  return (
    <div
      ref={box}
      class="term"
      tabIndex={0}
      onMouseDown={() => setFocused(props.id)}
      onKeyDown={(e) => {
        const bytes = encodeKey(e);
        if (bytes === null) return;
        e.preventDefault();
        api.scroll(props.id, 0).catch(() => {});
        send(bytes);
      }}
      onPaste={(e) => {
        const text = e.clipboardData?.getData("text") ?? "";
        if (text) {
          e.preventDefault();
          send(paste(text));
        }
      }}
      onWheel={(e) => {
        e.preventDefault();
        const lines = Math.round(e.deltaY / metrics().h) || Math.sign(e.deltaY);
        api.scroll(props.id, lines * 3).catch(() => {});
      }}
    >
      <canvas ref={canvas} />
    </div>
  );
}
