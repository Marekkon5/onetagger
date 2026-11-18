import assert from 'node:assert/strict';
import test from 'node:test';
import { resolveLocale, russianPluralRule, cardinalPluralRule } from '../client/src/i18n/locale.ts';

test('explicit supported preferences override the browser and accept regional tags', () => {
    for (const locale of ['en', 'it', 'fr', 'nl', 'de', 'es', 'el', 'ru']) {
        assert.equal(resolveLocale(locale, ['en-US']), locale);
    }
    assert.equal(resolveLocale('fr-CH', ['de-CH']), 'fr');
    assert.equal(resolveLocale('DE-ch', ['fr-FR']), 'de');
    assert.equal(resolveLocale('ru-RU', ['en-US']), 'ru');
});

test('first supported browser language is used only when no preference was saved', () => {
    assert.equal(resolveLocale(undefined, ['ja-JP', 'fr-CH', 'de-DE']), 'fr');
    assert.equal(resolveLocale(null, ['zh-CN', 'el-GR']), 'el');
    assert.equal(resolveLocale(undefined, ['nl-NL']), 'nl');
    assert.equal(resolveLocale(undefined, ['ja-JP']), 'en');
    assert.equal(resolveLocale(undefined, []), 'en');
});

test('unsupported and corrupt saved settings safely select the English fallback', () => {
    for (const value of ['', 'xx', 'ja-JP', '<script>', 'fr/../../ru', {}, [], 123, false]) {
        assert.equal(resolveLocale(value, ['fr-CH']), 'en', `unexpected preference ${JSON.stringify(value)}`);
    }
});

test('Russian cardinal grammar includes teen exceptions and fractional other form', () => {
    const cases: [number, number][] = [
        [0, 2], [1, 0], [2, 1], [3, 1], [4, 1], [5, 2], [11, 2], [12, 2],
        [14, 2], [20, 2], [21, 0], [22, 1], [25, 2], [101, 0], [111, 2],
        [1.5, 3], [2.1, 3], [-1, 0], [-22, 1],
    ];
    for (const [count, index] of cases) {
        assert.equal(russianPluralRule(count, 4), index, `Russian grammar for ${count}`);
    }
});

test('Russian rules remain within shorter catalog alternative lists', () => {
    for (const count of [0, 1, 2, 5, 11, 21, 22, 25, 1.5]) {
        assert.equal(russianPluralRule(count, 1), 0);
        const two = russianPluralRule(count, 2);
        assert.ok(two === 0 || two === 1);
        assert.equal(two, new Intl.PluralRules('ru').select(count) === 'one' ? 0 : 1);
        assert.ok(russianPluralRule(count, 3) >= 0 && russianPluralRule(count, 3) < 3);
    }
});

test('other locales use their own cardinal grammar instead of English-only rules', () => {
    assert.equal(cardinalPluralRule('fr', 0, 2), 0);
    assert.equal(cardinalPluralRule('en', 0, 2), 1);
    for (const locale of ['en', 'it', 'nl', 'de', 'es', 'el'] as const) {
        assert.equal(cardinalPluralRule(locale, 1, 2), 0);
        assert.equal(cardinalPluralRule(locale, 2, 2), 1);
    }
    assert.equal(cardinalPluralRule('fr', 0, 3), 0); // Explicit zero form.
    assert.equal(cardinalPluralRule('fr', 1, 3), 1);
    assert.equal(cardinalPluralRule('fr', 2, 3), 2);
});
