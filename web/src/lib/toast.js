let handler = null;

export function registerToast(fn) {
  handler = fn;
}

export function showToast(message) {
  handler?.(message);
}

export async function copyText(text) {
  if (!text?.trim()) return false;
  try {
    await navigator.clipboard.writeText(text);
    showToast('Copied to clipboard');
    return true;
  } catch {
    showToast('Copy failed');
    return false;
  }
}