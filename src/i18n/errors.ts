import { t } from './index';

export function translateError(err: unknown): string {
  if (err === null || err === undefined) {
    return '';
  }

  if (typeof err === 'string') {
    return err;
  }

  if (err instanceof Error) {
    return err.message;
  }

  if (typeof err === 'object' && err !== null) {
    const obj = err as Record<string, unknown>;
    if (typeof obj.code === 'string') {
      const code = obj.code;
      const detail = typeof obj.detail === 'string' ? obj.detail : '';
      const translationKey = `errors.${code}`;
      const translated = t(translationKey, detail);
      if (translated !== translationKey) {
        return translated;
      }
      return code;
    }
  }

  return String(err);
}