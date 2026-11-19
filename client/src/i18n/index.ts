import { createI18n } from 'vue-i18n';
import { watch } from 'vue';
import { Quasar } from 'quasar';
import enUS from 'quasar/lang/en-US';
import it from 'quasar/lang/it';
import fr from 'quasar/lang/fr';
import nl from 'quasar/lang/nl';
import de from 'quasar/lang/de';
import es from 'quasar/lang/es';
import el from 'quasar/lang/el';
import ru from 'quasar/lang/ru';
import ptPT from 'quasar/lang/pt';
import ptBR from 'quasar/lang/pt-BR';
import cs from 'quasar/lang/cs';
import pl from 'quasar/lang/pl';
import ja from 'quasar/lang/ja';
import ar from 'quasar/lang/ar';
import { LOCALE_STORAGE_KEY, SUPPORTED_LOCALES, resolveLocale, russianPluralRule, cardinalPluralRule, type SupportedLocale } from './locale';

export { LOCALE_STORAGE_KEY, SUPPORTED_LOCALES, resolveLocale, russianPluralRule };
export type { SupportedLocale };

type Catalog = Record<string, string>;
const messages = Object.fromEntries(SUPPORTED_LOCALES.map(locale => [locale, {}])) as
    Record<SupportedLocale, Record<string, Catalog>>;
const catalogs = import.meta.glob<Catalog>('../locales/*/*.json', { eager: true, import: 'default' });
for (const [path, catalog] of Object.entries(catalogs)) {
    const match = path.match(/\/([a-z]{2}(?:-[A-Z]{2})?)\/([a-zA-Z0-9_-]+)\.json$/);
    if (!match || !SUPPORTED_LOCALES.includes(match[1] as SupportedLocale)) continue;
    messages[match[1] as SupportedLocale][match[2]] = catalog;
}

function initialLocale(): SupportedLocale {
    let saved: string | null = null;
    try { saved = window.localStorage.getItem(LOCALE_STORAGE_KEY); } catch { /* Storage may be disabled. */ }
    return resolveLocale(saved, navigator.languages?.length ? navigator.languages : [navigator.language]);
}

export const quasarLanguagePacks = {
    en: enUS, it, fr, nl, de, es, el, ru,
    'pt-PT': ptPT, 'pt-BR': ptBR, cs, pl, ja, ar,
};
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

applyDocumentLocale(i18n.global.locale.value as SupportedLocale);
watch(i18n.global.locale, value => {
    const locale = value as SupportedLocale;
    applyDocumentLocale(locale);
    Quasar.lang.set(quasarLanguagePacks[locale]);
}, { flush: 'sync' });

export function setLocale(value: unknown): void {
    const locale = resolveLocale(value);
    i18n.global.locale.value = locale;
    try { window.localStorage.setItem(LOCALE_STORAGE_KEY, locale); } catch { /* Keep the session usable without storage. */ }
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
