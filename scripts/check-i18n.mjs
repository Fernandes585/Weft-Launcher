import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const i18nDir = join(root, "src", "i18n");
const locales = ["en", "pt-BR", "es", "ru"];

const read = (locale) =>
  JSON.parse(readFileSync(join(i18nDir, `${locale}.json`), "utf8"));

/** Flattens nested message groups into `group.key` paths. */
const flatten = (value, prefix = "") =>
  Object.entries(value).flatMap(([key, child]) => {
    const path = prefix ? `${prefix}.${key}` : key;
    const isGroup =
      child !== null && typeof child === "object" && !Array.isArray(child);
    return isGroup ? flatten(child, path) : [[path, child]];
  });

const reference = new Map(flatten(read("en")));
const referenceKeys = [...reference.keys()].sort();
let failed = false;

for (const locale of locales.slice(1)) {
  const keys = new Set(flatten(read(locale)).map(([key]) => key));

  for (const key of referenceKeys) {
    if (!keys.has(key)) {
      console.error(`${locale}: missing key "${key}"`);
      failed = true;
    }
  }
  for (const key of keys) {
    if (!reference.has(key)) {
      console.error(`${locale}: extra key "${key}"`);
      failed = true;
    }
  }
}

for (const locale of locales) {
  for (const [key, value] of flatten(read(locale))) {
    if (typeof value !== "string" || value.trim() === "") {
      console.error(`${locale}: key "${key}" must be a non-empty string`);
      failed = true;
    }
  }
}

if (failed) {
  process.exit(1);
}
console.log(`i18n OK: ${referenceKeys.length} keys in ${locales.length} locales`);
