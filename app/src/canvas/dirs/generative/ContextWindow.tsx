// The artboard for the context window: the message box with its panel open above.

import Composer from "../../ui/Composer";

/** The artboard: the message box with its context panel open above it. */
export default function ContextWindow() {
  return (
    <div data-dir="gen" class="flex h-full w-full flex-col justify-end p-7">
      <Composer contextOpen />
    </div>
  );
}
