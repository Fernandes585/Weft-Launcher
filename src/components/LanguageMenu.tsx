import { useEffect, useRef, useState } from "react";
import { getLocale, setLocale, Locale } from "../i18n";

const OPTIONS: { code: Locale; label: string; name: string }[] = [
  { code: "en", label: "EN", name: "English" },
  { code: "pt-BR", label: "BR", name: "Português" },
  { code: "es", label: "ES", name: "Español" },
  { code: "ru", label: "RU", name: "Русский" },
];

export function LanguageMenu() {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const current = getLocale();
  const currentLabel = OPTIONS.find((o) => o.code === current)?.label ?? "EN";

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

  const choose = (code: Locale) => {
    setOpen(false);
    if (code === current) return;
    setLocale(code);
    window.location.reload();
  };

  return (
    <div className="lang" ref={ref}>
      <button
        type="button"
        className="lang-btn"
        onClick={() => setOpen((o) => !o)}
        aria-haspopup="listbox"
        aria-expanded={open}
      >
        {currentLabel}
      </button>
      {open && (
        <ul className="lang-menu" role="listbox">
          {OPTIONS.map((o) => (
            <li key={o.code} role="option" aria-selected={o.code === current}>
              <button
                type="button"
                className={`lang-item${o.code === current ? " active" : ""}`}
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