// One icon per role of the crew (Phosphor), in a soft tile. The tile carries the
// agent's state: a ring that breathes while it works, the one hue while it waits
// for the user, a small check when it is done.

import { Show } from "solid-js";
import { cx } from "@/lib/cva";
import Icon from "../Icon";
import { ic } from "../ph";
import { roleHint, type Role, type Visual } from "./roles";

export const roleIcon: Record<Role, string> = {
  orchestrator: ic.compass,
  planner: ic.mapTrifold,
  teacher: ic.graduation,
  worker: ic.hammer,
  scout: ic.binoculars,
  reviewer: ic.sealCheck,
};
export const roleLabel = roleHint;

type Props = { role: Role; state?: Visual; size?: "sm" | "md" | "lg"; class?: string };

export default function AgentIcon(props: Props) {
  const size = () => props.size ?? "md";
  const px = () => ({ sm: 13, md: 15, lg: 20 })[size()];
  return (
    <span
      role="img"
      aria-label={`${props.role}${props.state ? `, ${props.state}` : ""}`}
      class={cx(
        "relative grid shrink-0 place-items-center bg-muted text-foreground transition-[background-color,box-shadow,color] duration-300",
        size() === "sm" && "size-6 rounded-md",
        size() === "md" && "size-7 rounded-lg",
        size() === "lg" && "size-10 rounded-xl",
        props.state === "working" && "ring-1 ring-foreground/40",
        props.state === "blocked" && "bg-signal/10 text-signal ring-1 ring-signal/60",
        props.state === "idle" && "text-muted-foreground",
        props.state === "done" && "text-muted-foreground",
        props.class,
      )}
    >
      <Icon icon={roleIcon[props.role]} size={px()} />
      <Show when={props.state === "working"}><span class="m-ringout absolute inset-0 rounded-[inherit] ring-1 ring-foreground/30" /></Show>
      <Show when={props.state === "done"}>
        <span class="m-pop absolute -right-0.5 -bottom-0.5 grid size-3 place-items-center rounded-full bg-foreground text-background"><svg viewBox="0 0 16 16" class="size-2"><path d="M3.5 8.5l3 3 6-7" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" /></svg></span>
      </Show>
    </span>
  );
}
