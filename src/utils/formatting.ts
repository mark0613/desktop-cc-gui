export function formatDownloadSize(bytes: number | null | undefined) {
  if (!bytes || bytes <= 0) {
    return "0 MB";
  }
  const gb = bytes / (1024 ** 3);
  if (gb >= 1) {
    const digits = gb >= 10 ? 0 : 1;
    return `${gb.toFixed(digits)} GB`;
  }
  const mb = bytes / (1024 ** 2);
  const digits = mb >= 10 ? 0 : 1;
  return `${mb.toFixed(digits)} MB`;
}

export function formatByteSize(bytes: number | null | undefined) {
  if (!bytes || !Number.isFinite(bytes) || bytes <= 0) {
    return null;
  }

  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unitIndex = 0;

  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  const maximumFractionDigits = unitIndex === 0 || value >= 100 ? 0 : 1;
  return `${new Intl.NumberFormat(undefined, { maximumFractionDigits }).format(value)} ${units[unitIndex]}`;
}
