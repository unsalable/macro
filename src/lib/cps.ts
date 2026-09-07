export const MIN_INTERVAL_MS = 1;
export const MAX_INTERVAL_MS = 3_600_000;

/** interval <-> CPS is a two-way binding; intervalMs is the single source of truth (§11). */
export function intervalToCps(intervalMs: number): number {
  if (intervalMs <= 0) return 0;
  return 1000 / intervalMs;
}

export function cpsToInterval(cps: number): number {
  if (cps <= 0) return MAX_INTERVAL_MS;
  return clampInterval(1000 / cps);
}

export function clampInterval(intervalMs: number): number {
  return Math.min(MAX_INTERVAL_MS, Math.max(MIN_INTERVAL_MS, intervalMs));
}

export function formatCps(cps: number): string {
  if (cps >= 100) return cps.toFixed(0);
  if (cps >= 10) return cps.toFixed(1);
  return cps.toFixed(2);
}
