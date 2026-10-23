// The trust scan before a crew starts. A line sweeps the card while files are
// read; findings rise one by one; a blocking finding holds the start until the
// user accepts, which records a fingerprint and releases the crew.

import { For, Show, createEffect, createSignal } from "solid-js";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { cx } from "@/lib/cva";
import Icon from "../../ui/Icon";
import { ic } from "../icons";
import { createSim } from "../ui/sim";
import { trust } from "./cardData";

export default function TrustCard() {
  const sim = createSim(1000, 11, 4);
  const t = sim.t;
  const [accepted, setAccepted] = createSignal(false);
  createEffect(() => {
    if (t() === 0) setAccepted(false);
    if (t() === 8) setAccepted(true);
  });
  const scanning = () => t() < 2;
  const found = () => trust.findings.filter((_, i) => t() >= 2 + i);
  return (
    <Card class="gap-0 overflow-hidden rounded-xl py-0 shadow-none">
      <header class={cx("flex items-center gap-2.5 border-b px-4 py-3", scanning() && "m-sweep")}>
        <span class="grid size-7 place-items-center rounded-lg bg-muted"><Icon icon={accepted() ? ic.sealCheck : ic.shield} size={16} /></span>
        <div class="flex-1 leading-tight"><p class="text-sm font-medium">Trust scan</p><p class="num text-xs text-muted-foreground">{scanning() ? `Reading ${trust.files} files` : `${trust.files} files read · ${found().length} of ${trust.findings.length} findings`}</p></div>
        <Show when={accepted()} fallback={<Show when={!scanning()}><span class="m-pop inline-flex items-center gap-1.5 text-xs"><span class="m-ring size-1.5 rounded-full bg-signal" />Blocks the start</span></Show>}>
          <span class="m-pop inline-flex items-center gap-1 text-xs text-muted-foreground"><Icon icon={ic.checkBold} size={12} />Accepted</span>
        </Show>
      </header>
      <ul class="flex min-h-[148px] flex-col divide-y">
        <For each={found()}>
          {(f, i) => (
            <li class="m-rise flex gap-3 px-4 py-3" style={{ "--i": 0 }}>
              <span class={cx("mt-0.5 inline-flex h-5 shrink-0 items-center rounded-full px-2 text-[11px] font-medium", f.sev === "High" ? "bg-foreground text-background" : "bg-muted text-foreground")}>{f.sev}</span>
              <div class="min-w-0 flex-1 text-sm leading-snug">
                <p class="font-mono text-xs text-muted-foreground">{f.file}</p>
                <p>{f.msg}</p>
              </div>
            </li>
          )}
        </For>
      </ul>
      <footer class="flex h-12 items-center gap-3 border-t px-4 text-sm">
        <Show when={accepted()} fallback={<span class="text-muted-foreground">{scanning() ? "Scanning…" : "Review both findings to start"}</span>}>
          <span class="m-rise inline-flex items-center gap-2"><Icon icon={ic.fingerprint} size={15} />Fingerprint <span class="font-mono text-xs">{trust.fingerprint}</span> saved · crew can start</span>
        </Show>
        <Button size="sm" class="ml-auto h-8" disabled={scanning() || accepted()} onClick={() => setAccepted(true)}>Accept and continue</Button>
      </footer>
    </Card>
  );
}
