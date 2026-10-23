import { render } from "solid-js/web";

async function main() {
  // Dev-only: ?mock fakes the Tauri backend to work on the UI in a browser.
  if (import.meta.env.DEV && location.search.includes("mock")) {
    await import("./dev/mock");
    // ?view=chat&theme=light opens the mock on a view and a theme (used for screenshots).
    const q = new URLSearchParams(location.search);
    const [{ setView }, { setTheme }] = await Promise.all([import("./store"), import("./ui/state")]);
    if (q.get("view")) setView(q.get("view") as Parameters<typeof setView>[0]);
    if (q.get("theme")) setTheme(q.get("theme") as "light" | "dark");
  }
  // Dev-only: ?canvas opens the design canvas instead of the app.
  const { default: App } = await (import.meta.env.DEV && location.search.includes("canvas") ? import("./canvas/Canvas") : import("./appEntry"));
  render(() => <App />, document.getElementById("root")!);
}
main();
