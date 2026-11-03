export function summarizeNotes(body: string, maxLines = 6): string {
  const lines = body.split(/\r?\n/).map((l) => l.trim());

  // Match section headers with optional # markdown prefix
  const headerPattern = /^(#{1,6}\s*)?(what'?s?\s*changed|changes|changelog|novidades|alterações|fixes|features|updates|melhorias|correções|novos)[\s:]*$/i;

  const startIdx = lines.findIndex((l) => headerPattern.test(l));

  const relevantLines = startIdx >= 0 ? lines.slice(startIdx + 1) : lines;

  return relevantLines
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
