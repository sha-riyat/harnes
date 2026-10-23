// What waits for the user: a typed question and a permission. The same anatomy
// in every direction; the direction decides where it sits.

import { For, createSignal } from "solid-js";
import { Button } from "@/components/ui/button";
import { RadioGroup, RadioGroupItem, RadioGroupItemControl, RadioGroupItemIndicator, RadioGroupItemInput, RadioGroupItemLabel, RadioGroupItems } from "@/components/ui/radio-group";
import { cx } from "@/lib/cva";
import { permission, question } from "../data";

export function AskCard(props: { class?: string; inline?: boolean }) {
  const [choice, setChoice] = createSignal("smtp");
  return (
    <div class={cx("flex flex-col gap-3", props.class)}>
      <p class="text-sm leading-snug font-medium">{question.text}</p>
      <RadioGroup value={choice()} onChange={setChoice} aria-label="Answer">
        <RadioGroupItems class={props.inline ? "grid grid-cols-2 gap-2" : "flex-col gap-2"}>
          <For each={question.options}>
            {(o) => (
              <RadioGroupItem value={o.id} class="rounded-lg border px-3 py-2.5 transition-colors hover:bg-accent data-[checked]:border-foreground">
                <RadioGroupItemInput />
                <RadioGroupItemControl>
                  <RadioGroupItemIndicator />
                </RadioGroupItemControl>
                <RadioGroupItemLabel class="flex flex-col gap-0 leading-tight">
                  {o.label}
                  <span class="text-xs font-normal text-muted-foreground">{o.detail}</span>
                </RadioGroupItemLabel>
              </RadioGroupItem>
            )}
          </For>
        </RadioGroupItems>
      </RadioGroup>
      <div class="flex items-center gap-2">
        <Button size="sm">Send answer</Button>
        <Button size="sm" variant="ghost" class="text-muted-foreground">Skip</Button>
      </div>
    </div>
  );
}

export function PermissionRow(props: { class?: string }) {
  return (
    <div class={cx("flex flex-col gap-2", props.class)}>
      <p class="text-sm leading-snug font-medium">{permission.agent} wants to run a command</p>
      <code class="rounded-md bg-muted px-2.5 py-1.5 font-mono text-xs">{permission.command}</code>
      <div class="flex items-center gap-2">
        <Button size="sm">Allow once</Button>
        <Button size="sm" variant="outline">Always</Button>
        <Button size="sm" variant="ghost" class="text-muted-foreground">Deny</Button>
      </div>
    </div>
  );
}
