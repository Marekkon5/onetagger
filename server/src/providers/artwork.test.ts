import assert from 'node:assert/strict';
import test from 'node:test';
import { ArtworkFetchError, fetchProviderArtwork } from './artwork.js';

test('fetchProviderArtwork accepts bounded images from provider hosts', async (context) => {
  const calls: string[] = [];
  context.mock.method(globalThis, 'fetch', async (input: string | URL | Request) => {
    calls.push(String(input));
    return new Response(new Uint8Array([1, 2, 3]), { headers: { 'content-type': 'image/jpeg' } });
  });

  const result = await fetchProviderArtwork('https://i.discogs.com/cover.jpg');

  assert.equal(result.contentType, 'image/jpeg');
  assert.deepEqual([...result.bytes], [1, 2, 3]);
  assert.deepEqual(calls, ['https://i.discogs.com/cover.jpg']);
});

test('fetchProviderArtwork rejects unapproved hosts and redirect targets', async (context) => {
  const fetchMock = context.mock.method(globalThis, 'fetch', async () => new Response(null, {
    status: 302,
    headers: { location: 'http://127.0.0.1/private' },
  }));

  await assert.rejects(() => fetchProviderArtwork('https://i.discogs.com/cover.jpg'), ArtworkFetchError);
  await assert.rejects(() => fetchProviderArtwork('https://example.com/cover.jpg'), /approved provider image host/);
  assert.equal(fetchMock.mock.callCount(), 1);
});

test('fetchProviderArtwork rejects non-image responses', async (context) => {
  context.mock.method(globalThis, 'fetch', async () => new Response('not an image', {
    headers: { 'content-type': 'text/html' },
  }));

  await assert.rejects(() => fetchProviderArtwork('https://archive.org/cover.jpg'), /not a supported image/);
});