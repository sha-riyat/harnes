// A sidebar row: icon or dot, label, and a count only on a queue that needs action.

import { Show } from "solid-js";
import { cx } from "@/lib/cva";
import Icon from "../../ui/Icon";

export default function SideRow(props: { icon?: string; label: string; on?: boolean; count?: number; dot?: boolean; onClick?: () => void }) {
  return (
    <button onClick={props.onClick} class={cx("flex h-8 w-full items-center gap-2.5 rounded-lg px-2.5 text-left text-sm text-muted-foreground transition-colors hover:bg-accent/60 hover:text-foreground", props.on && "bg-accent text-foreground")}>
      <Show when={props.icon}><Icon icon={props.icon!} size={16} /></Show>
      <Show when={props.dot}><span class="size-2 rounded-full border border-muted-foreground" /></Show>
      <span class="min-w-0 flex-1 truncate">{props.label}</span>
      <Show when={props.count}><span class="num grid h-5 min-w-5 place-items-center rounded-full bg-foreground px-1.5 text-[11px] font-medium text-background">{props.count}</span></Show>
    </button>
  );
}
