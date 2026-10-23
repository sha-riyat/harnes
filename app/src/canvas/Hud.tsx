// Fixed controls: zoom, fit, reset camera, and the one global theme selector.

import { For } from "solid-js";
import { theme, setTheme, type Theme } from "../ui/state";

const THEMES: Theme[] = ["system", "light", "dark"];

type Props = {
  zoom: () => number;
  onZoom: (dir: 1 | -1) => void;
  onFit: () => void;
  onReset: () => void;
  onTidy: () => void;
};

export default function Hud(props: Props) {
  return (
    <>
      <div class="cv-hud" role="toolbar" aria-label="Canvas view">
        <button onClick={() => props.onZoom(-1)} aria-label="Zoom out" title="Zoom out (-)">−</button>
        <output aria-live="off" title="Zoom">{props.zoom()}%</output>
        <button onClick={() => props.onZoom(1)} aria-label="Zoom in" title="Zoom in (+)">+</button>
        <span class="cv-sep" />
        <button onClick={props.onFit} title="Fit all (F)">Fit all</button>
        <button onClick={props.onReset} title="Reset camera (0)">Reset</button>
        <button onClick={props.onTidy} title="Put every item back where the source places it">Tidy up</button>
      </div>
      <div class="cv-theme" role="radiogroup" aria-label="Theme">
        <For each={THEMES}>
          {(t) => (
            <button role="radio" aria-checked={theme() === t} classList={{ on: theme() === t }} onClick={() => setTheme(t)}>
              {t}
            </button>
          )}
        </For>
      </div>
      <p id="cv-move-help" class="cv-sr">Arrow keys move the item by 10 pixels. Hold Shift for 50.</p>
    </>
  );
}
