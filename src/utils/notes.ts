export function summarizeNotes(body: string, maxLines = 6): string {
  return body
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => l && !/^[-*_]{3,}$/.test(l))
    .map((l) =>
      l
        .replace(/^#{1,6}\s*/, "")
        .replace(/\*\*|__|`/g, "")
        .replace(/^[-*]\s+/, "• ")
    )
    .slice(0, maxLines)
    .join("\n");
}
