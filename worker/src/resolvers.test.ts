import assert from 'node:assert/strict';
import test from 'node:test';
import { PROVIDER_CAPABILITIES } from '@syco23/catalog-domain';
import {
  RESOLVER_PROVIDERS,
  archiveOrgIdentifier,
  discogsResource,
  firstAudioHref,
  freeteknomusicIdentity,
  hearthisPath,
  isUploadUrn,
  mixcloudKey,
  musicbrainzResource,
  youtubeVideoId,
} from './resolvers.js';

test('every metadata-capable provider has a worker resolver', () => {
  for (const [provider, capability] of Object.entries(PROVIDER_CAPABILITIES)) {
    if (!capability.metadata) continue;
    assert.ok(
      (RESOLVER_PROVIDERS as readonly string[]).includes(provider),
      `provider ${provider} advertises metadata but has no worker resolver`,
    );
  }
  assert.ok((RESOLVER_PROVIDERS as readonly string[]).includes('user_upload'));
});

test('provider URL parsers accept canonical shapes and reject foreign hosts', () => {
  assert.equal(youtubeVideoId('https://www.youtube.com/watch?v=dQw4w9WgXcQ'), 'dQw4w9WgXcQ');
  assert.equal(youtubeVideoId('https://youtu.be/dQw4w9WgXcQ'), 'dQw4w9WgXcQ');
  assert.equal(youtubeVideoId('https://example.com/watch?v=dQw4w9WgXcQ'), undefined);
  assert.equal(youtubeVideoId('https://www.youtube.com/watch?v=short'), undefined);

  assert.equal(hearthisPath('https://hearthis.at/artist/track/'), 'artist/track/');
  assert.equal(hearthisPath('https://evil.example/artist/track/'), undefined);

  assert.equal(mixcloudKey('https://www.mixcloud.com/artist/show/'), '/artist/show/');
  assert.equal(mixcloudKey('https://evil.example/artist/show/'), undefined);

  assert.equal(archiveOrgIdentifier('https://archive.org/details/some_item'), 'some_item');
  assert.equal(archiveOrgIdentifier('https://archive.org/download/some_item/file.mp3'), 'some_item');
  assert.equal(archiveOrgIdentifier('https://evil.example/details/some_item'), undefined);

  assert.deepEqual(discogsResource('https://www.discogs.com/release/249504-Rick-Astley-Never-Gonna-Give-You-Up'), { kind: 'release', id: '249504' });
  assert.deepEqual(discogsResource('https://www.discogs.com/master/12345'), { kind: 'master', id: '12345' });
  assert.equal(discogsResource('https://www.discogs.com/artist/123'), undefined);
  assert.equal(discogsResource('https://evil.example/release/249504'), undefined);

  assert.deepEqual(
    musicbrainzResource('https://musicbrainz.org/recording/b0261007-c152-424e-8c17-339ba94c0f3d'),
    { kind: 'recording', mbid: 'b0261007-c152-424e-8c17-339ba94c0f3d' },
  );
  assert.deepEqual(
    musicbrainzResource('https://musicbrainz.org/release-group/095a6fdc-c932-4590-9d91-bd4578dd62d2/'),
    { kind: 'release-group', mbid: '095a6fdc-c932-4590-9d91-bd4578dd62d2' },
  );
  assert.equal(musicbrainzResource('https://musicbrainz.org/artist/aad88397-a12a-40ca-9091-de4e0d2574e4'), undefined);
  assert.equal(musicbrainzResource('https://evil.example/recording/b0261007-c152-424e-8c17-339ba94c0f3d'), undefined);
});

test('freeteknomusic identity comes from the file path, never a placeholder', () => {
  assert.deepEqual(
    freeteknomusicIdentity('https://archive.freeteknomusic.org/Artist%20Name/Artist%20Name%20-%20Live%20Set.mp3'),
    { title: 'Live Set', artists: ['Artist Name'] },
  );
  assert.deepEqual(
    freeteknomusicIdentity('https://archive.freeteknomusic.org/Some_Artist/live_mix.mp3'),
    { title: 'live mix', artists: ['Some Artist'] },
  );
  assert.throws(() => freeteknomusicIdentity('https://archive.freeteknomusic.org/'), /usable file name/);
});

test('directory listings yield the first same-origin audio file', () => {
  const html = `
    <a href="../">Parent directory</a>
    <a href="notes.txt">notes</a>
    <a href="set-one.mp3">set one</a>
    <a href="https://evil.example/set-two.mp3">foreign</a>
  `;
  assert.equal(
    firstAudioHref(html, 'https://archive.freeteknomusic.org/Artist/'),
    'https://archive.freeteknomusic.org/Artist/set-one.mp3',
  );
  assert.equal(firstAudioHref('<a href="cover.jpg">cover</a>', 'https://archive.freeteknomusic.org/Artist/'), undefined);
});

test('upload URNs stay distinguishable from public URLs', () => {
  assert.equal(isUploadUrn('urn:syco23:upload:abc123'), true);
  assert.equal(isUploadUrn('https://example.com/mix.mp3'), false);
});
