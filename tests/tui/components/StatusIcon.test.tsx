import { describe, it, expect } from "bun:test";

describe("StatusIcon", () => {
  it("can be imported", async () => {
    const mod = await import("@/tui/components/StatusIcon");
    expect(mod.StatusIcon).toBeDefined();
    expect(typeof mod.StatusIcon).toBe("function");
  });
});
