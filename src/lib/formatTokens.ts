export function formatTokenCount(value: number): string {
  if (value >= 1_000_000) {
    const millions = value / 1_000_000;
    return Number.isInteger(millions) ? `${millions}M` : `${millions.toFixed(1)}M`;
  }
  if (value >= 10_000) {
    const thousands = Math.round(value / 1000);
    return `${thousands}k`;
  }
  if (value >= 1000) {
    const thousands = value / 1000;
    return `${thousands.toFixed(1)}k`;
  }
  return String(Math.max(0, Math.round(value)));
}
