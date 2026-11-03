import { describe, expect, it } from 'vitest';
import {
  fieldLabel,
  humanValue,
  providerLabel,
  statePillLabel,
  stateTone,
  formatTimestamp,
  timeAgo
} from './presentation';

describe('presentation vocabulary', () => {
  it('labels internal field keys and falls back to readable camelCase', () => {
    expect(fieldLabel('durationMs')).toBe('Duration');
    expect(fieldLabel('cover')).toBe('Cover art');
    expect(fieldLabel('artists')).toBe('Performers');
    expect(fieldLabel('possibleDuplicate')).toBe('Possible duplicate');
    expect(fieldLabel('someNewKey')).toBe('Some New Key');
  });

  it('never renders object values as raw JSON', () => {
    expect(humanValue(undefined)).toBe('Not recorded');
    expect(humanValue([])).toBe('Not recorded');
    expect(humanValue(true)).toBe('Yes');
    expect(humanValue(['a', 'b'])).toBe('a · b');
    expect(humanValue({ value: '2024', precision: 'year' })).toBe('2024 (year precision)');
    expect(humanValue({ recordId: 'record_12345678', revision: 2 })).toBe('Another record · revision 2');
    const summarized = humanValue({ title: 'Mix', genres: ['techno'] });
    expect(summarized).not.toContain('{');
    expect(summarized).not.toContain('JSON');
  });

  it('resolves provider names and state labels/tone consistently', () => {
    expect(providerLabel('soundcloud')).toBe('SoundCloud');
    expect(statePillLabel('job', 'running')).toBe('Searching…');
    expect(statePillLabel('disposition', 'selected')).toBe('Selected');
    expect(stateTone('health', 'ready')).toBe('ok');
    expect(stateTone('job', 'error')).toBe('danger');
    expect(stateTone('disposition', 'pending')).toBe('warn');
  });

  it('formats timestamps without throwing and keeps unparseable input readable', () => {
    expect(typeof formatTimestamp('2026-10-05T12:00:00.000Z')).toBe('string');
    expect(formatTimestamp('not-a-date')).toBe('not-a-date');
    expect(typeof timeAgo('2026-10-05T12:00:00.000Z')).toBe('string');
    expect(timeAgo(undefined)).toBe('—');
  });
});
