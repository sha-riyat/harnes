// One line, drawn to the app-screen rules: straight segments, a few faint grid
// lines, round ticks above the maximum, no y axis, the event that matters named.

import { For } from "solid-js";
import { NOW, time, windowUsage } from "../data";

const MAX = 40;
const TICKS = [0, 10, 20, 30, 40];
const HOURS = [7, 8, 9, 10, 11];

export default function UsageChart(props: { w?: number; h?: number; class?: string }) {
  const W = props.w ?? 640;
  const H = props.h ?? 200;
  const L = 34;
  const T = 26;
  const B = 24;
  const R = 8;
  const { start, end, points, events } = windowUsage;
  const x = (m: number) => L + ((m - start) / (end - start)) * (W - L - R);
  const y = (v: number) => T + (1 - v / MAX) * (H - T - B);
  const path = points.map(([m, v], i) => `${i ? "L" : "M"}${x(m).toFixed(1)},${y(v).toFixed(1)}`).join(" ");
  const last = points[points.length - 1];

  return (
    <svg viewBox={`0 0 ${W} ${H}`} class={props.class} width="100%" role="img" aria-label={`Usage over the five-hour window, ${last[1]}% at ${time(NOW)}`}>
      <g class="num fill-muted-foreground text-[11px]">
        <For each={TICKS}>
          {(t) => (
            <>
              <line x1={L} x2={W - R} y1={y(t)} y2={y(t)} class={t === 0 ? "stroke-foreground/40" : "stroke-border"} stroke-width="1" />
              <text x={L - 8} y={y(t) + 4} text-anchor="end">{t}%</text>
            </>
          )}
        </For>
        <For each={HOURS}>{(h) => <text x={x(h * 60)} y={H - 6} text-anchor="middle">{String(h).padStart(2, "0")}:00</text>}</For>
      </g>
      <For each={events}>
        {(e) => (
          <g>
            <line x1={x(e.at)} x2={x(e.at)} y1={T - 6} y2={H - B} class="stroke-muted-foreground/60" stroke-width="1" />
            <text x={x(e.at) - 6} y={T - 10} text-anchor="end" class="fill-muted-foreground text-[11px]">{e.label} {time(e.at)}</text>
          </g>
        )}
      </For>
      <path d={path} fill="none" class="stroke-foreground" stroke-width="2" stroke-linejoin="round" />
      <circle cx={x(last[0])} cy={y(last[1])} r="3.5" class="fill-foreground" />
      <text x={x(last[0]) + 9} y={y(last[1]) + 4} class="num fill-foreground text-[12px] font-medium">{last[1]}%</text>
    </svg>
  );
}
