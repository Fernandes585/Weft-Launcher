import { useState, useEffect, useCallback } from 'react';
import { t as translate, initI18n, getLocale, setLocale, formatSize as i18nFormatSize, Locale } from './index';

export function useI18n() {
  const [ready, setReady] = useState(false);

  useEffect(() => {
    initI18n();
    setReady(true);
  }, []);

  const t = useCallback((key: string, ...args: string[]) => translate(key, ...args), []);
  const formatSize = useCallback((bytes: number) => i18nFormatSize(bytes), []);

  const locale = getLocale();
  const changeLocale = useCallback((newLocale: Locale) => {
    setLocale(newLocale);
    window.location.reload();
  }, []);

  return { t, formatSize, locale, changeLocale, ready };
}