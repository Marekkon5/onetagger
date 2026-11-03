import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import type { MixRecord } from '@syco23/catalog-domain';
import { openCatalog } from './repository.js';
import { persistWaveform, readWaveformMedia } from './media.js';

const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+ip1sAAAAASUVORK5CYII=', 'base64');

function withCatalog(run: (directory: string, path: string, repo: ReturnType<typeof openCatalog>, closeRepo: () => void) => void) {
  const directory = mkdtempSync(join(tmpdir(), 'syco23-media-'));
  const path = join(directory, 'catalog.sqlite');
  const repo = openCatalog(path);
  const record: MixRecord = {
    kind: 'mix', id: 'mix_01J9CATALOGUE00000000000060', createdAt: '2026-10-03T12:00:00.000Z',
    updatedAt: '2026-10-03T12:00:00.000Z', revision: 1, verification: 'source-confirmed', reviewState: 'ready',
    title: 'Waveform source', people: [], eventIds: [], genres: [], styles: [], assets: [], sources: []
  };
  repo.transaction((tx) => tx.saveRecord(record));
  let closed = false;
  const closeRepo = () => {
    if (closed) return;
    repo.close();
    closed = true;
  };
  try { run(directory, path, repo, closeRepo); }
  finally { closeRepo(); rmSync(directory, { recursive: true, force: true }); }
}

test('waveform PNG files and media references survive a database reopen', () => {
  withCatalog((directory, path, repo, closeRepo) => {
    process.env.CATALOG_MEDIA_PATH = join(directory, 'media');
    try {
      const record = persistWaveform(repo, 'mix_01J9CATALOGUE00000000000060', png, 'https://archive.org/download/example/audio.mp3');
      const asset = record.kind === 'mix' ? record.assets[0] : undefined;
      assert.equal(asset?.role, 'waveform');
      assert.match(asset?.url ?? '', /^\/api\/catalog\/media\//);
      closeRepo();
      const reopened = openCatalog(path);
      try { assert.deepEqual(readWaveformMedia(reopened, asset?.mediaId ?? ''), png); }
      finally { reopened.close(); }
    } finally {
      delete process.env.CATALOG_MEDIA_PATH;
    }
  });
});

test('waveform storage rejects non-PNG data and oversized images', () => {
  withCatalog((_directory, _path, repo) => {
    assert.throws(() => persistWaveform(repo, 'mix_01J9CATALOGUE00000000000060', Buffer.from('not png'), 'https://archive.org/audio.mp3'), /PNG/i);
    assert.throws(() => persistWaveform(repo, 'mix_01J9CATALOGUE00000000000060', Buffer.alloc(9 * 1024 * 1024), 'https://archive.org/audio.mp3'), /size/i);
  });
});
test('waveform storage rejects header-only and corrupt PNG payloads',()=>{
  withCatalog((_directory,_path,repo)=>{
    assert.throws(()=>persistWaveform(repo,'mix_01J9CATALOGUE00000000000060',png.subarray(0,24),'https://archive.org/audio.mp3'),/PNG/i);
    const corrupt=Buffer.from(png);corrupt[corrupt.length-1]^=1;
    assert.throws(()=>persistWaveform(repo,'mix_01J9CATALOGUE00000000000060',corrupt,'https://archive.org/audio.mp3'),/PNG/i);
  });
});
