export function parseTag(s: string): number[] {
  return s.replace(/^v/i, '').split(/[.\-]/).map((n) => parseInt(n, 10) || 0);
}

export function compareTags(a: string, b: string): number {
  const x = parseTag(a);
  const y = parseTag(b);
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    const d = (x[i] || 0) - (y[i] || 0);
    if (d) return d;
  }
  return 0;
}

/**
 * Returns the highest version tag from an array of installed versions.
 * Returns null if the array is empty.
 */
export function getHighestInstalledTag(installedVersions: { tag: string }[]): string | null {
  if (installedVersions.length === 0) return null;
  return [...installedVersions].sort((a, b) => compareTags(b.tag, a.tag))[0].tag;
}

/**
 * A release is "new" if:
 * - It is NOT installed
 * - There is at least one installed version
 * - Its tag is greater than the highest installed tag
 */
export function isNewRelease(
  releaseTag: string,
  installedVersions: { tag: string }[],
  installedMap: Map<string, { tag: string }>
): boolean {
  if (installedMap.has(releaseTag)) return false; // already installed
  const highestInstalled = getHighestInstalledTag(installedVersions);
  if (!highestInstalled) return false; // no versions installed yet
  return compareTags(releaseTag, highestInstalled) > 0;
}