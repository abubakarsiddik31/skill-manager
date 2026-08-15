/** Compact "3h ago"-style label for a unix-seconds timestamp. */
export function relativeTime(unixSeconds: number): string {
  if (!unixSeconds) return "unknown";
  const seconds = Math.max(0, Date.now() / 1000 - unixSeconds);
  const minutes = seconds / 60;
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${Math.floor(minutes)}m ago`;
  const hours = minutes / 60;
  if (hours < 24) return `${Math.floor(hours)}h ago`;
  const days = hours / 24;
  if (days < 30) return `${Math.floor(days)}d ago`;
  const months = days / 30;
  if (months < 12) return `${Math.floor(months)}mo ago`;
  return `${Math.floor(months / 12)}y ago`;
}
