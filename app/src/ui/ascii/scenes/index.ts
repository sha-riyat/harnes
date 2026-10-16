// Every scene of the field, organic (things passing between voices) and
// geometric (things being drawn) alternating, so one motif hands over to a
// different kind.

import { BUILD } from "./build";
import type { MakeScene } from "./core";
import { TALK } from "./talk";

export const SCENES: MakeScene[] = TALK.flatMap((t, i) => (BUILD[i] ? [t, BUILD[i]] : [t]));
export { EXTRA_GLYPHS, type Cell, type Grid, type Scene } from "./core";
