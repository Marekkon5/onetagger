import assert from 'node:assert/strict';
import test from 'node:test';
import { HearthisProvider, hearthisPath } from './hearthis.js';

test('hearthis.at accepts direct track links and rejects foreign hosts', () => {
  assert.equal(hearthisPath('https://hearthis.at/artist/long-mix/?foo=1'), 'artist/long-mix/');
  assert.equal(hearthisPath('https://hearthis.at.evil.test/artist/long-mix/'), undefined);
});

test('hearthis.at uses track artwork and rejects uploader avatars', async (context) => {
  const payload = {
    id: '123', title: 'Long mix', duration: '3600', permalink_url: 'https://hearthis.at/artist/long-mix/',
    artwork_url: 'https://img.hearthis.at/uploads/1/image_track/123/cover.jpg', user: { username: 'Artist' },
  };
  const fetchMock = context.mock.method(globalThis, 'fetch', async () => Response.json(payload));
  const provider = new HearthisProvider();
  const candidates = await provider.search({ url: payload.permalink_url });
  assert.equal(candidates[0].durationMs, 3_600_000);
  assert.equal(candidates[0].artwork?.[0], payload.artwork_url);
  fetchMock.mock.mockImplementation(async () => Response.json({ ...payload, artwork_url: 'https://img.hearthis.at/uploads/1/image_user/avatar.jpg' }));
  assert.equal(await provider.lookupArtwork(payload.permalink_url), undefined);
});
