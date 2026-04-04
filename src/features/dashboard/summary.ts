export type ProviderStatus =
  | "healthy"
  | "warning"
  | "danger"
  | "needs_setup"
  | "error"
  | "stale";

export type ProviderSnapshotVm = {
  provider: "zhipu" | "minimax" | "kimi";
  status: ProviderStatus;
  headlineValue: string;
  fetchedAt: string;
  isStale: boolean;
  message: string | null;
};

type SummaryTone = "healthy" | "warning" | "danger" | "muted";

type StatusSummary = {
  primaryProvider: ProviderSnapshotVm["provider"] | null;
  compactText: string;
  tone: SummaryTone;
};

const providerOrder: ProviderSnapshotVm["provider"][] = ["zhipu", "minimax", "kimi"];
const statusPriority: Record<ProviderStatus, number> = {
  danger: 5,
  error: 4,
  warning: 3,
  stale: 2,
  healthy: 1,
  needs_setup: 0,
};
const providerTag: Record<ProviderSnapshotVm["provider"], string> = {
  zhipu: "Z",
  minimax: "M",
  kimi: "K",
};

export function buildStatusSummary(snapshots: ProviderSnapshotVm[]): StatusSummary {
  if (snapshots.length === 0) {
    return {
      primaryProvider: null,
      compactText: "配置套餐",
      tone: "muted",
    };
  }

  const primaryProvider =
    [...snapshots].sort(
      (left, right) => statusPriority[right.status] - statusPriority[left.status],
    )[0]?.provider ?? null;

  const compactText = providerOrder
    .map((provider) => {
      const snapshot = snapshots.find((entry) => entry.provider === provider);
      return `${providerTag[provider]}${snapshot?.headlineValue ?? "--"}`;
    })
    .join(" ");

  const tone = resolveTone(snapshots);

  return {
    primaryProvider,
    compactText,
    tone,
  };
}

function resolveTone(snapshots: ProviderSnapshotVm[]): SummaryTone {
  const highestPriority = Math.max(...snapshots.map((snapshot) => statusPriority[snapshot.status]));

  if (highestPriority >= statusPriority.danger) {
    return "danger";
  }

  if (highestPriority >= statusPriority.warning) {
    return "warning";
  }

  if (highestPriority === 0) {
    return "muted";
  }

  return "healthy";
}
