/** Parse recording timestamps from API or filename-derived strings. */
export function parseRecordingTs(ts) {
  if (!ts) return null;

  const iso = ts.includes(' ') ? ts.replace(' ', 'T') : ts;
  let d = new Date(iso);
  if (!isNaN(d.getTime())) return d;

  const m = ts.match(/^(\d{4}-\d{2}-\d{2})[T ](\d{2})-(\d{2})-(\d{2})/);
  if (m) {
    d = new Date(`${m[1]}T${m[2]}:${m[3]}:${m[4]}`);
    if (!isNaN(d.getTime())) return d;
  }

  return null;
}

export function formatRecordingTime(ts) {
  const d = parseRecordingTs(ts);
  if (!d) return ts || '';
  return d.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
}

export function relativeRecordingTime(ts) {
  const d = parseRecordingTs(ts);
  if (!d) return ts || '';

  const diff = Date.now() - d.getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(diff / 3600000);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(diff / 86400000);
  if (days < 7) return `${days}d ago`;
  if (days < 30) return `${days}d ago`;
  const weeks = Math.floor(days / 7);
  if (weeks < 5) return `${weeks}w ago`;
  return d.toLocaleDateString();
}

/** Parse API duration (`120s`) or raw seconds. */
export function parseDurationSecs(value) {
  if (typeof value === 'number' && value >= 0) return Math.floor(value);
  const m = String(value ?? '').match(/^(\d+)s$/);
  return m ? parseInt(m[1], 10) : 0;
}

/** Human-readable length: 45s, 2m 15s, 1h 5m. */
export function formatDuration(value) {
  const secs = parseDurationSecs(value);
  if (secs < 1) return '<1s';
  if (secs < 60) return `${secs}s`;
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  if (m < 60) return s ? `${m}m ${s}s` : `${m}m`;
  const h = Math.floor(m / 60);
  const rm = m % 60;
  if (rm && secs % 60) return `${h}h ${rm}m ${secs % 60}s`;
  if (rm) return `${h}h ${rm}m`;
  return `${h}h`;
}

/** Primary label + opposite tooltip for history/detail timestamps. */
export function recordingTimeLabels(ts, mode = 'relative') {
  const absolute = formatRecordingTime(ts);
  const relative = relativeRecordingTime(ts);
  if (mode === 'absolute') {
    return { primary: absolute, tooltip: relative };
  }
  return { primary: relative, tooltip: absolute };
}