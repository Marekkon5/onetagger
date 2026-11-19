export const SUPPORTED_LOCALES = ['en', 'it', 'fr', 'nl', 'de', 'es', 'el', 'ru',
    'pt-PT', 'pt-BR', 'cs', 'pl', 'ja', 'ar'] as const;
export type SupportedLocale = typeof SUPPORTED_LOCALES[number];
export const LOCALE_STORAGE_KEY = 'onetagger.locale';

function supported(value: unknown): SupportedLocale | undefined {
    if (typeof value !== 'string') return undefined;
    // Accept language tags, not arbitrary paths or expressions. Region-specific
    // variants use the corresponding bundled language catalog.
    const normalized = value.trim().replace(/_/g, '-');
    if (!/^[a-z]{2,3}(?:-[a-z0-9]{2,8})*$/i.test(normalized)) return undefined;
    const language = normalized.split('-')[0].toLowerCase();
    if (language === 'pt') {
        return /^pt(?:-[a-z]{4})?-br(?:-|$)/i.test(normalized) ? 'pt-BR' : 'pt-PT';
    }
    return SUPPORTED_LOCALES.find(locale => locale === language);
}

export function resolveLocale(preferred?: unknown, browserLanguages: readonly string[] = []): SupportedLocale {
    if (preferred !== undefined && preferred !== null) return supported(preferred) ?? 'en';
    for (const candidate of browserLanguages) {
        const locale = supported(candidate);
        if (locale) return locale;
    }
    return 'en';
}

const cardinalRules = Object.fromEntries(SUPPORTED_LOCALES.map(locale => [locale, new Intl.PluralRules(locale)]));
const slavicForms = ['one', 'few', 'many', 'other'];
const arabicForms = ['zero', 'one', 'two', 'few', 'many', 'other'];

export function cardinalPluralRule(locale: SupportedLocale, choice: number, choicesLength: number): number {
    if (choicesLength <= 1) return 0;
    const category = cardinalRules[locale].select(choice);
    if (choicesLength === 2) return category === 'one' ? 0 : 1;
    if (locale === 'ru' || locale === 'cs' || locale === 'pl') {
        return Math.min(slavicForms.indexOf(category), choicesLength - 1);
    }
    if (locale === 'ar') return Math.min(arabicForms.indexOf(category), choicesLength - 1);
    // Three-form messages explicitly provide zero | one | other.
    return choice === 0 ? 0 : category === 'one' ? 1 : 2;
}

/** Four-form catalog order: one | few | many | other (including fractions). */
export function russianPluralRule(choice: number, choicesLength: number): number {
    return cardinalPluralRule('ru', choice, choicesLength);
}
