// The run at a glance: the one question first (does the crew need me?), the run as a
// strip, what the orchestrator did, what the agents said, the commits, and on the
// right what waits for the user. Everything is read from the run's real state.

import { Badge } from "@/components/ui/badge";
import { needCount } from "../../model";
import NeedsYou from "../cards/Decisions";
import { ActivityCard, CommitsCard, GateCard, LeadCard, WorkCard } from "../cards/Overview";
import ThreadsCard from "../cards/Threads";

export default function Overview() {
  return (
    <div class="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_360px] gap-4 overflow-y-auto p-5">
      <div class="flex flex-col gap-4 [&>*]:shrink-0">
        <LeadCard />
        <div class="grid h-[280px] grid-cols-2 gap-4"><WorkCard /><ThreadsCard /></div>
        <CommitsCard />
      </div>
      <aside class="flex flex-col gap-3 [&>*]:shrink-0">
        <div class="flex items-center justify-between px-1"><h2 class="text-sm font-semibold">Needs you</h2><Badge variant="secondary" class="num">{needCount()}</Badge></div>
        <NeedsYou />
        <GateCard />
        <ActivityCard />
      </aside>
    </div>
  );
}
