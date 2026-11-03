import { describe, expect, it } from 'vitest';
import { applyEnrichment, applyFilenameIdentity, AUTO_ACCEPT_EVIDENCE, buildTaggedMp3, hasSolidMetadataBase, normalizeTagText, parseFilenameIdentity, providerFromSourceUrl, type LocalMp3Track } from './local-mp3';

function track(overrides: Partial<LocalMp3Track> = {}): LocalMp3Track {
  return {
    id: 'folder/set.mp3:100:1',
    file: new File([new Uint8Array([1, 2, 3])], 'set.mp3', { type: 'audio/mpeg' }),
    relativePath: 'set.mp3',
    title: 'Untitled',
    artists: [],
    artistInput: '',
    album: '',
    albumArtist: '',
    genres: [],
    comment: '',
    composer: [],
    label: [],
    key: '',
    lyrics: '',
    mood: '',
    compilation: false,
    entities: [],
    enrichedBy: [],
    state: 'scanned',
    detail: 'Tag scan complete',
    ...overrides,
  };
}

describe('local MP3 tags', () => {
  it('normalizes text and requires a meaningful title plus artist for enrichment', () => {
    expect(normalizeTagText('  Metek\u0000  Live   1998  ')).toBe('Metek Live 1998');
    expect(hasSolidMetadataBase(track({ title: 'Track 01', artists: ['Metek'] }))).toBe(false);
    expect(hasSolidMetadataBase(track({ title: 'Live in Prague', artists: ['Metek'] }))).toBe(true);
  });

  it('fills only missing fields from confident provider results', () => {
    const item = track({ title: 'Live set', artists: ['Metek'], genres: ['Tekno'], year: 1998 });
    const added = applyEnrichment(item, {
      patch: {
        recordedAt: '2001-01-01',
        description: '  Recorded   live. ',
        genres: ['Freetekno'],
        artwork: [{ url: 'https://i.discogs.com/cover.jpg', provider: 'discogs', kind: 'cover' }],
        sources: [{ provider: 'archiveorg', url: 'https://archive.org/details/set' }],
      },
      candidates: [],
      entities: [{ kind: 'artist', name: 'Metek', provider: 'discogs', externalId: '123', url: 'https://www.discogs.com/artist/123' }],
      provenance: [
        { provider: 'archiveorg', field: 'description', confidence: 0.84 },
        { provider: 'archiveorg', field: 'artwork', confidence: 0.84 },
      ],
      attempted: ['archiveorg'],
      failures: [],
    });

    expect(added).toBe(4);
    expect(item.year).toBe(1998);
    expect(item.genres).toEqual(['Tekno']);
    expect(item.comment).toBe('Recorded live.');
    expect(item.coverUrl).toBe('https://i.discogs.com/cover.jpg');
    expect(item.sourceUrl).toBe('https://archive.org/details/set');
    expect(item.entities[0]?.externalId).toBe('123');
    expect(item.enrichedBy).toEqual(['archiveorg', 'discogs']);
  });

  it('derives identity and an evidence rate from a good filename', () => {
    const strong = parseFilenameIdentity('Kan10 - Live Mackitek Koalisson III 2013.mp3');
    expect(strong.artists).toEqual(['Kan10']);
    expect(strong.title).toBe('Live Mackitek Koalisson III');
    expect(strong.year).toBe(2013);
    expect(strong.evidence).toBeGreaterThanOrEqual(AUTO_ACCEPT_EVIDENCE);

    const threshold = parseFilenameIdentity('Metek - Live in Prague.mp3');
    expect(threshold.artists).toEqual(['Metek']);
    expect(threshold.title).toBe('Live in Prague');
    expect(threshold.evidence).toBe(AUTO_ACCEPT_EVIDENCE);
    expect(parseFilenameIdentity('track01.mp3').evidence).toBeLessThan(AUTO_ACCEPT_EVIDENCE);
  });

  it('auto-accepts a good filename but never overwrites a curator edit', () => {
    const scanned = track({ title: 'Metek - Live in Prague', relativePath: 'Metek - Live in Prague.mp3' });
    applyFilenameIdentity(scanned);
    expect(scanned.title).toBe('Live in Prague');
    expect(scanned.artists).toEqual(['Metek']);
    expect(scanned.accepted).toBe(true);

    const noisy = track({ title: 'track01', relativePath: 'track01.mp3' });
    applyFilenameIdentity(noisy);
    expect(noisy.accepted).toBe(false);

    const edited = track({ title: 'My chosen title', artists: ['Curator'], relativePath: 'Metek - Live in Prague.mp3' });
    applyFilenameIdentity(edited);
    expect(edited.title).toBe('My chosen title');
    expect(edited.artists).toEqual(['Curator']);
  });

  it('maps a public source URL to a provider identity and rejects local paths', () => {
    expect(providerFromSourceUrl('https://www.mixcloud.com/keja/live/')).toBe('mixcloud');
    expect(providerFromSourceUrl('https://archive.org/details/example')).toBe('archiveorg');
    expect(providerFromSourceUrl('https://www.youtube.com/watch?v=abc')).toBe('youtube');
    expect(providerFromSourceUrl('/Users/me/Music/set.mp3')).toBeUndefined();
    expect(providerFromSourceUrl(undefined)).toBeUndefined();
  });

  it('builds a new tagged copy without modifying the source buffer', () => {
    const source = new Uint8Array([0xff, 0xfb, 0x90, 0x64]).buffer;
    const item = track({ title: 'Live in Prague', artists: ['Metek'], genres: ['Freetekno'] });
    const output = buildTaggedMp3(source, item);

    expect(output.size).toBeGreaterThan(source.byteLength);
    expect([...new Uint8Array(source)]).toEqual([0xff, 0xfb, 0x90, 0x64]);
  });
});