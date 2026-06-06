/** Merge a fresh first-page poll into the current history list (newest first). */
export function mergeHistoryEntries(current, fresh) {
  if (!fresh?.length) return current;

  const freshMap = new Map(fresh.map((e) => [e.file, e]));
  const merged = fresh.map((e) => freshMap.get(e.file));

  for (const entry of current) {
    if (!freshMap.has(entry.file)) {
      merged.push(entry);
    }
  }

  return merged;
}