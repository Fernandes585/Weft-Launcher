import { useEffect, useRef, useState } from "react";
import { useI18n } from "../i18n/useI18n";

const OPTIONS: { code: 'pt-BR' | 'en' | 'es' | 'ru'; label: string; name: string }[] = [
  { code: "en", label: "EN", name: "English" },
  { code: "pt-BR", label: "BR", name: "Português" },
  { code: "es", label: "ES", name: "Español" },
  { code: "ru", label: "RU", name: "Русский" },
];

export function LanguageMenu() {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const { locale, changeLocale } = useI18n();

  useEffect(() => {
    if (!open) return;
    const onClick = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onClick);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onClick);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const choose = (code: 'pt-BR' | 'en' | 'es' | 'ru') => {
    setOpen(false);
    if (code === locale) return;
    changeLocale(code);
  };

  const currentLabel = OPTIONS.find((o) => o.code === locale)?.label ?? "EN";

  return (
    <div className="lang" ref={ref}>
      <button
        type="button"
        className="lang-btn"
        onClick={() => setOpen((o: boolean) => !o)}
        aria-haspopup="listbox"
        aria-expanded={open}
      >
        {currentLabel}
      </button>
      {open && (
        <ul className="lang-menu" role="listbox">
          {OPTIONS.map((o) => (
            <li key={o.code} role="option" aria-selected={o.code === locale}>
              <button
                type="button"
                className={`lang-item${o.code === locale ? " active" : ""}`}
                onClick={() => choose(o.code)}
              >
                <strong>{o.label}</strong> <span>{o.name}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}