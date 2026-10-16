import { render } from "solid-js/web";
import "./theme.css";
import "./styles.css";
import "./lookout.css";
import "./methods.css";
import "./threads.css";

async function main() {
  // Dev-only: ?mock fakes the Tauri backend to work on the UI in a browser.
  if (import.meta.env.DEV && location.search.includes("mock")) await import("./dev/mock");
  const { default: App } = await import("./App");
  render(() => <App />, document.getElementById("root")!);
}
main();
