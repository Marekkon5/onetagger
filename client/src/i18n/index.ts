import { createI18n } from 'vue-i18n';
import { Quasar, type QuasarLanguage } from 'quasar';
import { LOCALE_STORAGE_KEY, SUPPORTED_LOCALES, resolveLocale, russianPluralRule, cardinalPluralRule, type SupportedLocale } from './locale';

export { LOCALE_STORAGE_KEY, SUPPORTED_LOCALES, resolveLocale, russianPluralRule };
export type { SupportedLocale };

type Catalog = Record<string, string>;
type Catalogs = Record<string, Catalog>;
const messages = Object.fromEntries(SUPPORTED_LOCALES.map(locale => [locale, {}])) as Record<SupportedLocale, Catalogs>;
// One chunk per language; neither application catalogs nor Quasar packs are eager.
const catalogs = import.meta.glob<Catalogs>('./catalogs/*.ts', { import: 'default' });
const packs: Record<SupportedLocale, () => Promise<{ default: QuasarLanguage }>> = {
    'en': () => import('quasar/lang/en-US'),
    'it': () => import('quasar/lang/it'),
    'fr': () => import('quasar/lang/fr'),
    'nl': () => import('quasar/lang/nl'),
    'de': () => import('quasar/lang/de'),
    'es': () => import('quasar/lang/es'),
    'el': () => import('quasar/lang/el'),
    'ru': () => import('quasar/lang/ru'),
    'pt-PT': () => import('quasar/lang/pt'),
    'pt-BR': () => import('quasar/lang/pt-BR'),
    'cs': () => import('quasar/lang/cs'),
    'pl': () => import('quasar/lang/pl'),
    'ja': () => import('quasar/lang/ja'),
    'ar': () => import('quasar/lang/ar'),
};

function initialLocale(): SupportedLocale {
    let saved: string | null = null;
    try { saved = window.localStorage.getItem(LOCALE_STORAGE_KEY); } catch { /* Storage may be disabled. */ }
    return resolveLocale(saved, navigator.languages?.length ? navigator.languages : [navigator.language]);
}

export const i18n = createI18n({
    legacy: false,
    globalInjection: true,
    locale: initialLocale(),
    fallbackLocale: 'en',
    messages,
    pluralRules: Object.fromEntries(SUPPORTED_LOCALES.map(locale => [locale,
        (choice: number, choicesLength: number) => cardinalPluralRule(locale, choice, choicesLength)])),
    missingWarn: import.meta.env.DEV,
    fallbackWarn: import.meta.env.DEV,
    warnHtmlMessage: true,
    // Messages and arguments are rendered by Vue interpolation or text-only
    // Quasar dialogs, not HTML. Escaping here would display double-escaped text.
    escapeParameter: false,
});

function applyDocumentLocale(locale: SupportedLocale): void {
    document.documentElement.lang = locale;
    document.documentElement.dir = locale === 'ar' ? 'rtl' : 'ltr';
}

const loading = new Map<SupportedLocale, Promise<QuasarLanguage>>();
let selection = 0;

function loadLocale(locale: SupportedLocale): Promise<QuasarLanguage> {
    const existing = loading.get(locale);
    if (existing) return existing;
    const promise = Promise.all([catalogs[`./catalogs/${locale}.ts`](), packs[locale]()])
        .then(([catalog, pack]) => {
            messages[locale] = catalog;
            i18n.global.setLocaleMessage(locale, catalog);
            return pack.default;
        }).catch(error => { loading.delete(locale); throw error; });
    loading.set(locale, promise);
    return promise;
}

export async function initializeI18n(): Promise<QuasarLanguage> {
    const locale = initialLocale();
    const [, pack] = await Promise.all([loadLocale('en'), loadLocale(locale)]);
    i18n.global.locale.value = locale;
    applyDocumentLocale(locale);
    return pack;
}

export async function setLocale(value: unknown): Promise<void> {
    const locale = resolveLocale(value);
    const request = ++selection;
    const pack = await loadLocale(locale);
    // A slow response cannot undo a later language choice.
    if (request !== selection) return;
    i18n.global.locale.value = locale;
    Quasar.lang.set(pack);
    applyDocumentLocale(locale);
    try { window.localStorage.setItem(LOCALE_STORAGE_KEY, locale); } catch { /* Storage may be disabled. */ }
}

export const localeOptions: { label: string; value: SupportedLocale }[] = [
    { label: 'English', value: 'en' }, { label: 'Italiano', value: 'it' },
    { label: 'Français', value: 'fr' }, { label: 'Nederlands', value: 'nl' },
    { label: 'Deutsch', value: 'de' }, { label: 'Español', value: 'es' },
    { label: 'Ελληνικά', value: 'el' }, { label: 'Русский', value: 'ru' },
    { label: 'Português (Portugal)', value: 'pt-PT' }, { label: 'Português (Brasil)', value: 'pt-BR' },
    { label: 'Čeština', value: 'cs' }, { label: 'Polski', value: 'pl' },
    { label: '日本語', value: 'ja' }, { label: 'العربية', value: 'ar' },
];

export function translate(key: string, values: Record<string, string | number> = {}): string {
    return i18n.global.t(key, values);
}

/** Translate only our versioned message envelope; older servers/plugins keep
 * their original text. Never infer meaning by comparing English error strings. */
export function translateBackendMessage(event: {
    messageKey?: unknown; messageArgs?: unknown; message?: unknown; error?: unknown;
}): string {
    const key = event.messageKey;
    if (typeof key === 'string' && /^backend\.[a-zA-Z0-9_]+$/.test(key) && i18n.global.te(key, 'en')) {
        const args: Record<string, string | number> = Object.create(null);
        if (event.messageArgs && typeof event.messageArgs === 'object' && !Array.isArray(event.messageArgs)) {
            for (const [name, value] of Object.entries(event.messageArgs)) {
                if (/^[a-zA-Z][a-zA-Z0-9_]*$/.test(name) && !['constructor', 'prototype', '__proto__'].includes(name)
                    && (typeof value === 'string' || (typeof value === 'number' && Number.isFinite(value)))) {
                    args[name] = value;
                }
            }
        }
        const template = messages.en.backend?.[key.slice('backend.'.length)];
        // Malformed envelopes fall back intact instead of exposing unresolved
        // placeholders to users. Values themselves remain literal Unicode text.
        const required = template?.matchAll(/\{([a-zA-Z][a-zA-Z0-9_]*)\}/g) ?? [];
        if ([...required].every(match => Object.hasOwn(args, match[1]))) return translate(key, args);
    }
    const fallback = typeof event.message === 'string' ? event.message : event.error;
    return typeof fallback === 'string' ? fallback : translate('core.unexpectedError');
}
