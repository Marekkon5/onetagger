import assert from 'node:assert/strict';
import test from 'node:test';
import { MixcloudProvider, mixcloudKey } from './mixcloud.js';

test('mixcloud accepts direct cloudcast links and rejects foreign or non-track hosts', () => {
  assert.equal(mixcloudKey('https://www.mixcloud.com/kejamackitek/keja-live-tribecore/'), '/kejamackitek/keja-live-tribecore/');
  assert.equal(mixcloudKey('https://mixcloud.com.evil.test/user/set/'), undefined);
  assert.equal(mixcloudKey('https://mixcloud.com/onlyprofile/'), undefined);
});

const cloudcast = {
  key: '/kejamackitek/keja-live-tribecore/',
  name: 'Keja - Live Tribecore',
  url: 'https://www.mixcloud.com/kejamackitek/keja-live-tribecore/',
  audio_length: 3365,
  created_time: '2011-02-07T20:49:42Z',
  description: 'Live recording',
  tags: [{ name: 'Tribecore' }, { name: 'Hardtek' }],
  pictures: { large: 'https://thumbnailer.mixcloud.com/unsafe/300x300/extaudio/2/e/4/3/cover.jpg' },
  user: { name: 'Keja Mackitek', username: 'kejamackitek' },
};

test('mixcloud maps a public cloudcast without treating the uploader as a performer', async (context) => {
  context.mock.method(globalThis, 'fetch', async () => Response.json(cloudcast));
  const provider = new MixcloudProvider();
  const candidates = await provider.search({ url: cloudcast.url });
  assert.equal(candidates.length, 1);
  const [candidate] = candidates;
  assert.equal(candidate.durationMs, 3_365_000);
  assert.deepEqual(candidate.artists, []);
  assert.equal(candidate.uploader, 'Keja Mackitek');
  assert.equal(candidate.uploadedAt, cloudcast.created_time);
  // The upload date is never promoted to a recording date.
  assert.equal(candidate.recordedAt, undefined);
  assert.deepEqual(candidate.genres, ['Tribecore', 'Hardtek']);
  assert.equal(candidate.artwork?.[0], cloudcast.pictures.large);
  assert.equal(candidate.externalIds?.mixcloud, cloudcast.key);
});

test('mixcloud search filters short clips and rejects non-Mixcloud artwork hosts', async (context) => {
  const long = { ...cloudcast, key: '/a/long/', url: 'https://www.mixcloud.com/a/long/', name: 'Long', audio_length: 5400 };
  const short = { ...cloudcast, key: '/a/short/', url: 'https://www.mixcloud.com/a/short/', name: 'Short', audio_length: 120 };
  context.mock.method(globalThis, 'fetch', async () => Response.json({ data: [long, short] }));
  const provider = new MixcloudProvider();
  const candidates = await provider.search({ q: 'Live' });
  assert.deepEqual(candidates.map((candidate) => candidate.title), ['Long']);
  context.mock.method(globalThis, 'fetch', async () => Response.json({ ...long, pictures: { large: 'https://evil.example/cover.jpg' } }));
  assert.equal(await provider.lookupArtwork(long.url), undefined);
});
