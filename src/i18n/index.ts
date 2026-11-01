export type Locale = 'pt-BR' | 'en' | 'es' | 'ru';

interface I18nMessages {
  heroBadge: string;
  appTitle: string;
  refreshButton: string;
  loading: string;
  cacheBanner: string;
  errorPrefix: string;
  emptyState: string;
  versionTag: string;
  versionName: string;
  prereleaseBadge: string;
  publishedDate: string;
  assetInfo: string;
  sha256Label: string;
  changelogSummary: string;
  installButton: string;
  installedBadge: string;
  installing: string;
  downloading: string;
  verifying: string;
  extracting: string;
  finalizing: string;
  deleteButton: string;
  deleteForReal: string;
  confirmDelete: string;
  deleteTitle: string;
  playButton: string;
  playing: string;
  progressDownloading: string;
  progressVerifying: string;
  progressExtracting: string;
  progressFinalizing: string;
  errorHashMissing: string;
  errorHashMismatch: string;
  errorDownloadFailed: string;
  errorExtractFailed: string;
  errorAlreadyInstalled: string;
  errorNotInstalled: string;
  errorAlreadyRunning: string;
  errorExecutableNotFound: string;
  errorGameRunning: string;
  errorVersionInUse: string;
  deleteDisabledTooltip: string;
  playDisabledTooltip: string;
  tabHome: string;
  tabVersions: string;
  tabNotes: string;
  tabsLabel: string;
  instanceName: string;
  statusReady: string;
  statusPlaying: string;
  selectVersionLabel: string;
  selectVersionPlaceholder: string;
  selectVersionHint: string;
  noVersionsInstalled: string;
  goToVersionsTab: string;
  instanceCardTitle: string;
  versionLabel: string;
  installedAtLabel: string;
  executableLabel: string;
  linkDiscord: string;
  linkWebsite: string;
  releasedLabel: string;
  sizeLabel: string;
  inUseBadge: string;
  noChangelog: string;
  dismiss: string;
  errors: {
    network: string;
    io: string;
    serialization: string;
    httpHeader: string;
    cacheCorrupted: string;
    assetNotFound: string;
    missingDigest: string;
    rateLimited: string;
    forbidden: string;
    offlineNoCache: string;
  };
  sizeUnits: {
    B: string;
    KB: string;
    MB: string;
    GB: string;
  };
}

import ptBR from './pt-BR.json';
import en from './en.json';
import es from './es.json';
import ru from './ru.json';

const messagesMap: Record<Locale, I18nMessages> = {
  'pt-BR': ptBR,
  en,
  es,
  ru,
};

let currentLocale: Locale = 'en';
let messages: I18nMessages = messagesMap.en;

export function initI18n(locale?: Locale): void {
  let saved: string | null = null;
  try {
    saved = localStorage.getItem('locale');
  } catch {
    saved = null;
  }
  const candidate = (locale || saved) as Locale | null;
  currentLocale = candidate && candidate in messagesMap ? candidate : 'en';
  messages = messagesMap[currentLocale];
}

export function getLocale(): Locale {
  return currentLocale;
}

export function setLocale(locale: Locale): void {
  currentLocale = locale;
  try {
    localStorage.setItem('locale', locale);
  } catch {
    /* ignora */
  }
  messages = messagesMap[locale];
}

export function t(key: string, ...args: string[]): string {
  if (!messages) {
    return key;
  }
  
  const keys = key.split('.');
  let value: any = messages;
  
  for (const k of keys) {
    if (value && typeof value === 'object' && k in value) {
      value = value[k];
    } else {
      return key;
    }
  }
  
  if (typeof value === 'string') {
    return args.reduce((str, arg, i) => str.replace(`{${i}}`, arg), value);
  }
  
  return key;
}

export function formatSize(bytes: number): string {
  if (!messages) {
    const units = ['B', 'KB', 'MB', 'GB'];
    let size = bytes;
    let unitIndex = 0;
    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024;
      unitIndex++;
    }
    return `${size.toFixed(1)} ${units[unitIndex]}`;
  }
  
  const units = [messages.sizeUnits.B, messages.sizeUnits.KB, messages.sizeUnits.MB, messages.sizeUnits.GB];
  let size = bytes;
  let unitIndex = 0;
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }
  return `${size.toFixed(1)} ${units[unitIndex]}`;
}