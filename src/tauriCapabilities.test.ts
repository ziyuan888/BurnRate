import { describe, expect, it } from "vitest";

import capability from "../src-tauri/capabilities/default.json";

describe("Tauri window capabilities", () => {
  it("allows the popover window to hide itself from the frontend", () => {
    expect(capability.windows).toContain("popover");
    expect(capability.permissions).toContain("core:window:allow-hide");
  });
});
