import { describe, expect, it } from "vitest";
import { buildStatusSummary, type ProviderSnapshotVm } from "./summary";

const baseSnapshot = {
  fetchedAt: "2026-04-03T13:00:00Z",
  isStale: false,
  message: null,
} satisfies Omit<ProviderSnapshotVm, "provider" | "status" | "headlineValue">;

describe("buildStatusSummary", () => {
  it("prefers the most dangerous provider for the menu bar summary", () => {
    const result = buildStatusSummary([
      {
        ...baseSnapshot,
        provider: "zhipu",
        status: "healthy",
        headlineValue: "37%",
      },
      {
        ...baseSnapshot,
        provider: "minimax",
        status: "warning",
        headlineValue: "82%",
      },
      {
        ...baseSnapshot,
        provider: "kimi",
        status: "needs_setup",
        headlineValue: "--",
      },
    ]);

    expect(result.primaryProvider).toBe("minimax");
    expect(result.compactText).toBe("Z37% M82% K--");
    expect(result.tone).toBe("warning");
  });

  it("falls back to setup state when all providers are disconnected", () => {
    const result = buildStatusSummary([]);

    expect(result.primaryProvider).toBeNull();
    expect(result.compactText).toBe("配置套餐");
    expect(result.tone).toBe("muted");
  });
});
