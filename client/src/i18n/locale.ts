export const SUPPORTED_LOCALES = ['en', 'it', 'fr', 'nl', 'de', 'es', 'el', 'ru'] as const;
export type SupportedLocale = typeof SUPPORTED_LOCALES[number];
export const LOCALE_STORAGE_KEY = 'onetagger.locale';

function supported(value: unknown): SupportedLocale | undefined {
    if (typeof value !== 'string') return undefined;
    // Accept language tags, not arbitrary paths or expressions. Region-specific
    // variants use the corresponding bundled language catalog.
    const normalized = value.trim().replace(/_/g, '-');
    if (!/^[a-z]{2,3}(?:-[a-z0-9]{2,8})*$/i.test(normalized)) return undefined;
    const language = normalized.split('-')[0].toLowerCase();
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

const russianCardinal = new Intl.PluralRules('ru');
const cardinalRules = Object.fromEntries(SUPPORTED_LOCALES.map(locale => [locale, new Intl.PluralRules(locale)]));

export function cardinalPluralRule(locale: SupportedLocale, choice: number, choicesLength: number): number {
    if (locale === 'ru') return russianPluralRule(choice, choicesLength);
    if (choicesLength <= 1) return 0;
    const one = cardinalRules[locale].select(choice) === 'one';
    if (choicesLength === 2) return one ? 0 : 1;
    // Three-form messages explicitly provide zero | one | other.
    return choice === 0 ? 0 : one ? 1 : 2;
}

/** Four-form catalog order: one | few | many | other (including fractions). */
export function russianPluralRule(choice: number, choicesLength: number): number {
    const category = russianCardinal.select(choice);
    if (choicesLength <= 1) return 0;
    if (choicesLength === 2) return category === 'one' ? 0 : 1;
    const index = category === 'one' ? 0 : category === 'few' ? 1 : category === 'many' ? 2 : 3;
    return Math.min(index, choicesLength - 1);
}
