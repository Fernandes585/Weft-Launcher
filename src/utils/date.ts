export function formatDate(dateString: string, locale: string = "pt-BR"): string {
  const date = new Date(dateString);
  return date.toLocaleString(locale, {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}