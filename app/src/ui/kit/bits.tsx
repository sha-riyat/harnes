// Small marks shared by the app: the harnes "h", an initials disc, a state dot.

import { cx } from "@/lib/cva";
import markSvg from "../mark.svg?raw";
import type { Visual } from "./roles";

// The harnes pixel "h", drawn in the current colour without its black square.
const glyph = markSvg.replace(/<rect width="64"[^>]*\/>/, "").replaceAll("#f2f2f2", "currentColor");

export function Mark(props: { class?: string }) {
  return <svg class={cx("size-6", props.class)} viewBox="0 0 64 64" aria-label="harnes" innerHTML={glyph} />;
}

/** State as shape: ink dot working, signal ring needs you, grey idle, filled grey done. */
export function StateDot(props: { state: Visual; class?: string }) {
  return (
    <span
      role="img"
      aria-label={props.state}
      class={cx(
        "inline-block size-2 shrink-0 rounded-full",
        props.state === "working" && "bg-foreground animate-pulse",
        props.state === "blocked" && "bg-signal ring-[3px] ring-signal/25",
        props.state === "idle" && "border border-muted-foreground",
        props.state === "done" && "bg-muted-foreground",
        props.class,
      )}
    />
  );
}

export function Initials(props: { name: string; class?: string }) {
  const t = props.name.split(/[\s-]+/).map((w) => w[0] ?? "").join("").slice(0, 2).toUpperCase();
  return <span class={cx("grid size-6 shrink-0 place-items-center rounded-full bg-secondary text-[10px] font-medium text-secondary-foreground", props.class)}>{t}</span>;
}
