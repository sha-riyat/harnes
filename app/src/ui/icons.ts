// Phosphor icons (MIT), light weight. Imported as raw SVG so only the
// icons listed here end up in the bundle.

import agent from "@phosphor-icons/core/assets/light/robot-light.svg?raw";
import alert from "@phosphor-icons/core/assets/light/warning-light.svg?raw";
import arrowLeft from "@phosphor-icons/core/assets/light/arrow-left-light.svg?raw";
import check from "@phosphor-icons/core/assets/light/check-light.svg?raw";
import chevron from "@phosphor-icons/core/assets/light/caret-down-light.svg?raw";
import close from "@phosphor-icons/core/assets/light/x-light.svg?raw";
import expand from "@phosphor-icons/core/assets/light/arrows-out-simple-light.svg?raw";
import folder from "@phosphor-icons/core/assets/light/folder-simple-light.svg?raw";
import grid from "@phosphor-icons/core/assets/light/squares-four-light.svg?raw";
import journal from "@phosphor-icons/core/assets/light/list-bullets-light.svg?raw";
import lock from "@phosphor-icons/core/assets/light/lock-simple-light.svg?raw";
import machine from "@phosphor-icons/core/assets/light/binoculars-light.svg?raw";
import moon from "@phosphor-icons/core/assets/light/moon-light.svg?raw";
import orchestrator from "@phosphor-icons/core/assets/light/chat-circle-text-light.svg?raw";
import plus from "@phosphor-icons/core/assets/light/plus-light.svg?raw";
import question from "@phosphor-icons/core/assets/light/question-light.svg?raw";
import search from "@phosphor-icons/core/assets/light/magnifying-glass-light.svg?raw";
import send from "@phosphor-icons/core/assets/light/arrow-up-light.svg?raw";
import sidebar from "@phosphor-icons/core/assets/light/sidebar-simple-light.svg?raw";
import stop from "@phosphor-icons/core/assets/light/stop-light.svg?raw";
import sun from "@phosphor-icons/core/assets/light/sun-light.svg?raw";
import terminal from "@phosphor-icons/core/assets/light/terminal-window-light.svg?raw";
import thread from "@phosphor-icons/core/assets/light/hash-light.svg?raw";
import threads from "@phosphor-icons/core/assets/light/chats-light.svg?raw";
import usage from "@phosphor-icons/core/assets/light/chart-line-light.svg?raw";
import cost from "@phosphor-icons/core/assets/light/coins-light.svg?raw";

/** The inner markup of a 256x256 Phosphor icon. */
export type IconData = string;

const inner = (svg: string): IconData => svg.replace(/^<svg[^>]*>/, "").replace(/<\/svg>\s*$/, "");

export const I = {
  agent, alert, arrowLeft, check, chevron, close, cost, expand, folder, grid, journal, lock, machine, moon,
  orchestrator, plus, question, search, send, sidebar, stop, sun, terminal, thread, threads, usage,
} as const;

export const icons = Object.fromEntries(Object.entries(I).map(([k, v]) => [k, inner(v)])) as Record<keyof typeof I, IconData>;
