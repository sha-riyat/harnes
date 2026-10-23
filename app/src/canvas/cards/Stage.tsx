// Each card on its own artboard, with its own replay, for the canvas gallery.

import type { Component } from "solid-js";
import LiveWork from "../dirs/fusion/LiveWork";
import { ThinkingCard } from "../dirs/generative/Thinking";
import { createSim } from "../ui/sim";
import AgentsCard from "./AgentsCard";
import DiffCard from "./DiffCard";
import GateCard from "./GateCard";
import MergeCard from "./MergeCard";
import PermissionCard from "./PermissionCard";
import PlanCard from "./PlanCard";
import QuestionCard from "./QuestionCard";
import TestsCard from "./TestsCard";
import ThreadCard from "./ThreadCard";
import TrustCard from "./TrustCard";

const stage = (Card: Component) => () => (
  <div data-dir="gen" class="h-full w-full p-6">
    <Card />
  </div>
);

export const QuestionStage = stage(() => <QuestionCard demo />);
export const PermissionStage = stage(() => <PermissionCard demo />);
export const PlanStage = stage(PlanCard);
export const MergeStage = stage(MergeCard);
export const DiffStage = stage(DiffCard);
export const TestsStage = stage(TestsCard);
export const TrustStage = stage(TrustCard);
export const GateStage = stage(GateCard);
export const AgentsStage = stage(AgentsCard);
export const ThinkingStage = stage(() => <ThinkingCard />);
export const WorkStage = stage(() => {
  const sim = createSim(1000, 8, 4);
  return <LiveWork t={sim.t} />;
});
export const ThreadStage = stage(ThreadCard);
