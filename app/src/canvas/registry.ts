// The canvas content. Each entry has a stable id, a readable name, its size and
// first position. To add, rename, duplicate or remove a preview, edit this list.

import type { Component } from "solid-js";
import * as V from "./dirs/fusion/Variants";
import Fusion from "./dirs/fusion/Fusion";
import * as S from "./cards/Stage";
import ContextWindow from "./dirs/generative/ContextWindow";
import Frame from "./dirs/frame/Frame";
import Ledger from "./dirs/ledger/Ledger";
import Terminal from "./dirs/terminal/Terminal";

export type CanvasItem = { id: string; name: string; w: number; h: number; x: number; y: number; Component: Component };

const W = 1440;
const H = 900;
const GAP = 160;

const screens: CanvasItem[] = [
  { id: "overview-ledger", name: "Overview · A Ledger (light rail, one question first)", w: W, h: H, x: 0, y: 0, Component: Ledger },
  { id: "overview-terminal", name: "Overview · B Terminal (dark, dense, three columns)", w: W, h: H, x: 0, y: 0, Component: Terminal },
  { id: "cockpit-frame", name: "Cockpit · C Frame (tinted frame, chat on a sheet)", w: W, h: H, x: 0, y: 0, Component: Frame },
  { id: "cockpit-fusion", name: "Cockpit · D Fusion of A and C (Ledger overview in a Frame, live crew)", w: W, h: H, x: 0, y: 0, Component: Fusion },
  { id: "cockpit-fusion-chat", name: "D · Chat (cards in the stream, one message box)", w: W, h: H, x: 0, y: 0, Component: V.FusionChat },
  { id: "view-methods", name: "D · Methods", w: W, h: H, x: 0, y: 0, Component: V.FusionMethods },
  { id: "view-settings", name: "D · Settings", w: W, h: H, x: 0, y: 0, Component: V.FusionSettings },
  { id: "panel-notifications", name: "D · Bell opened (notifications)", w: W, h: H, x: 0, y: 0, Component: V.FusionNotifications },
  { id: "panel-profile", name: "D · Avatar opened (profile menu)", w: W, h: H, x: 0, y: 0, Component: V.FusionProfile },
].map((it, i) => ({ ...it, x: (i % 3) * (W + GAP), y: Math.floor(i / 3) * (H + GAP) }));

// Every card an agent can put in the conversation, each on its own artboard and
// replaying its own story. Six columns, 680 wide; heights follow the content.
const CW = 680;
const CG = 110;
const cardList: [id: string, name: string, h: number, C: Component, w?: number][] = [
  ["card-question", "Card · Question (answer folds the card, agent resumes)", 340, S.QuestionStage],
  ["card-permission", "Card · Permission (allow runs the command, output streams)", 360, S.PermissionStage],
  ["card-thinking", "Card · Thinking timeline", 520, S.ThinkingStage],
  ["card-plan", "Card · Plan (waves, dependencies, release)", 440, S.PlanStage],
  ["card-work", "Card · Work log (live steps)", 300, S.WorkStage],
  ["card-thread", "Card · Thread (long messages: digest, read all, own scroll)", 860, S.ThreadStage, 468],
  ["card-merge", "Card · Merge (conflicts, checks, merged)", 700, S.MergeStage],
  ["card-diff", "Card · Diff (written live)", 440, S.DiffStage],
  ["card-tests", "Card · Tests (suites tick in)", 340, S.TestsStage],
  ["card-trust", "Card · Trust scan (blocks until accepted)", 400, S.TrustStage],
  ["card-gate", "Card · Run finished (ready for you)", 400, S.GateStage],
  ["card-agents", "Card · Crew as icons", 540, S.AgentsStage],
  ["card-context", "Card · Context window with compact", 540, ContextWindow],
];
const ROW0 = Math.ceil(screens.length / 3) * (H + GAP) + 120;
const cards: CanvasItem[] = [];
{
  const y = Array(6).fill(ROW0); // next free y per column, filled by shortest column
  for (const [id, name, h, Component, w = CW] of cardList) {
    const c = y.indexOf(Math.min(...y));
    cards.push({ id, name, w, h, x: c * (CW + CG), y: y[c], Component });
    y[c] += h + 110;
  }
}

export const items: CanvasItem[] = [...screens, ...cards];

// Raise this when the source layout above changes: saved positions from an older
// layout are then dropped once, so nothing opens on top of anything else.
export const LAYOUT_REV = 5;
