// A popover that stays inside its preview (no portal), so it works on the canvas
// at any zoom. It opens with a short fade and a 3% scale from its corner, closes on
// Escape or a press outside, and returns focus to nothing it did not take.

import { Show, createEffect, onCleanup, type JSX } from "solid-js";
import { cx } from "@/lib/cva";

type Props = {
  open: boolean;
  onClose: () => void;
  align?: "left" | "right";
  above?: boolean;
  class?: string;
  label: string;
  children: JSX.Element;
};

export default function Pop(props: Props) {
  let panel!: HTMLDivElement;
  createEffect(() => {
    if (!props.open) return;
    const down = (e: PointerEvent) => {
      const t = e.target as Element;
      // The trigger lives in the same relative wrapper; a press on it toggles on its own.
      if (!panel.contains(t) && !panel.parentElement?.contains(t)) props.onClose();
    };
    const key = (e: KeyboardEvent) => e.key === "Escape" && props.onClose();
    document.addEventListener("pointerdown", down, true);
    document.addEventListener("keydown", key);
    onCleanup(() => {
      document.removeEventListener("pointerdown", down, true);
      document.removeEventListener("keydown", key);
    });
  });
  return (
    <Show when={props.open}>
      <div
        ref={panel}
        role="dialog"
        aria-label={props.label}
        class={cx(
          "m-menu absolute z-30 overflow-hidden rounded-xl border bg-popover text-popover-foreground shadow-lg",
          props.above ? "bottom-full mb-2 origin-bottom-left" : "top-full mt-2",
          props.align === "right" ? "right-0 origin-top-right" : "left-0 origin-top-left",
          props.class,
        )}
      >
        {props.children}
      </div>
    </Show>
  );
}
