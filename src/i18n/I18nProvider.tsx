import { createContext, useContext, useState, useEffect, useCallback, useMemo, ReactNode } from 'react';
import { initI18n, getLocale, setLocale, formatSize as i18nFormatSize, t as translate, Locale } from './index';

interface I18nContextValue {
  locale: Locale;
  t: (key: string, ...args: string[]) => string;
  formatSize: (bytes: number) => string;
  changeLocale: (newLocale: Locale) => void;
  ready: boolean;
}

const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(() => getLocale());
  const [ready, setReady] = useState(false);

  useEffect(() => {
    initI18n();
    setReady(true);
    // Sync with localStorage on init
    const saved = localStorage.getItem('locale');
    if (saved && (saved as Locale) !== locale) {
      setLocaleState(saved as Locale);
    }
  }, [locale]);

  const t = useCallback((key: string, ...args: string[]) => translate(key, ...args), []);
  const formatSize = useCallback((bytes: number) => i18nFormatSize(bytes), []);

  const changeLocale = useCallback((newLocale: Locale) => {
    setLocale(newLocale);
    setLocaleState(newLocale);
  }, []);

  const value = useMemo<I18nContextValue>(
    () => ({ locale, t, formatSize, changeLocale, ready }),
    [locale, t, formatSize, changeLocale, ready],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nContextValue {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error('useI18n must be used within an I18nProvider');
  }
  return context;
}