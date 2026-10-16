// Renders Hugeicons (@hugeicons/core-free-icons, MIT) in SolidJS.
// Icon data is a list of [tag, attributes] with camelCase attribute names.

import { For, type JSX } from "solid-js";
import { Dynamic } from "solid-js/web";

export type IconData = readonly (readonly [string, Readonly<Record<string, string | number>>])[];

const kebab = (k: string) => k.replace(/[A-Z]/g, (c) => "-" + c.toLowerCase());

function attrs(raw: Readonly<Record<string, string | number>>, stroke?: number): Record<string, string | number> {
  const out: Record<string, string | number> = {};
  for (const [k, v] of Object.entries(raw)) {
    if (k === "key") continue;
    out[kebab(k)] = k === "strokeWidth" && stroke !== undefined ? stroke : v;
  }
  return out;
}

export default function Icon(props: {
  icon: IconData;
  size?: number;
  stroke?: number;
  class?: string;
  label?: string;
  style?: JSX.CSSProperties;
}) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={props.size ?? 16}
      height={props.size ?? 16}
      viewBox="0 0 24 24"
      fill="none"
      class={`icon ${props.class ?? ""}`}
      style={props.style}
      role={props.label ? "img" : undefined}
      aria-label={props.label}
      aria-hidden={props.label ? undefined : true}
    >
      <For each={props.icon}>{([tag, a]) => <Dynamic component={tag} {...attrs(a, props.stroke)} />}</For>
    </svg>
  );
}
