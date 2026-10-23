// One item on the canvas: a name that copies, a handle that moves, and the live
// preview. The preview is mounted once and never remounted by canvas work.

import { Show, createSignal, onCleanup, type JSX } from "solid-js";
import { ic } from "./icons";
import Icon from "../ui/Icon";

export const BAR = 30;

type Props = {
  id: string;
  name: string;
  w: number;
  h: number;
  ref: (el: HTMLElement) => void;
  onHandleDown: (e: PointerEvent) => void;
  onHandleKey: (e: KeyboardEvent) => void;
  children: JSX.Element;
};

export default function Frame(props: Props) {
  const [state, setState] = createSignal<"idle" | "copied" | "failed">("idle");
  let timer: number | undefined;
  onCleanup(() => clearTimeout(timer));

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(props.name);
      setState("copied");
      timer = window.setTimeout(() => setState("idle"), 1400);
    } catch {
      // No clipboard access: show the name as selectable text instead.
      setState("failed");
    }
  };

  return (
    <section class="cv-item" ref={props.ref} data-id={props.id} style={{ width: `${props.w}px` }}>
      <header class="cv-bar" style={{ height: `${BAR}px` }} onPointerDown={(e) => e.target === e.currentTarget && props.onHandleDown(e)}>
        <span class="cv-handle" role="button" tabindex="0" aria-label={`Move ${props.name}`} aria-describedby="cv-move-help" title="Drag to move. Arrow keys move it too." onPointerDown={props.onHandleDown} onKeyDown={props.onHandleKey}>
          <Icon icon={ic.grid} size={12} />
        </span>
        <Show
          when={state() !== "failed"}
          fallback={<input class="cv-fallback" readOnly value={props.name} aria-label="Name, select and copy" onFocus={(e) => e.currentTarget.select()} onBlur={() => setState("idle")} ref={(el) => queueMicrotask(() => el.select())} />}
        >
          <button class="cv-name" onClick={copy} title="Click to copy this name">
            {props.name}
          </button>
        </Show>
        <span class="cv-copied" role="status" aria-live="polite">{state() === "copied" ? "Copied" : ""}</span>
      </header>
      <div class="cv-body" style={{ width: `${props.w}px`, height: `${props.h}px` }}>
        {props.children}
      </div>
    </section>
  );
}
