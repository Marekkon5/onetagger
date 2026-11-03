import assert from 'node:assert/strict';
import test from 'node:test';
import { YouTubeProvider, youtubeDurationMs, youtubeVideoId } from './youtube.js';

test('YouTube accepts only public video URL forms and parses long durations', () => {
  assert.equal(youtubeVideoId('https://youtu.be/vi5miMVpmuI?t=30'), 'vi5miMVpmuI');
  assert.equal(youtubeVideoId('https://www.youtube.com/watch?v=vi5miMVpmuI'), 'vi5miMVpmuI');
  assert.equal(youtubeVideoId('https://youtube.com.evil.test/watch?v=vi5miMVpmuI'), undefined);
  assert.equal(youtubeDurationMs('PT1H30M12S'), 5_412_000);
});

test('YouTube known URL supplies a real public thumbnail without an API key', async (context) => {
  // This exercises the credential-free oEmbed path, so it must not be diverted into the
  // Data API branch by an ambient key from the developer environment.
  const previousKey = process.env.YOUTUBE_API_KEY;
  delete process.env.YOUTUBE_API_KEY;
  context.mock.method(globalThis, 'fetch', async () => Response.json({
    title: 'Kan10 - Live @ Mackitek Koalisson III',
    author_name: 'History of Free Party',
    thumbnail_url: 'https://i.ytimg.com/vi/vi5miMVpmuI/hqdefault.jpg',
  }));
  try {
    const provider = new YouTubeProvider();
    const candidates = await provider.search({ url: 'https://youtu.be/vi5miMVpmuI' });
    assert.equal(candidates[0].artwork?.[0], 'https://i.ytimg.com/vi/vi5miMVpmuI/hqdefault.jpg');
    assert.equal(candidates[0].source.externalId, 'vi5miMVpmuI');
  } finally {
    if (previousKey === undefined) delete process.env.YOUTUBE_API_KEY;
    else process.env.YOUTUBE_API_KEY = previousKey;
  }
});
