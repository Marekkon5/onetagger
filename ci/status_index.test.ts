import assert from 'node:assert/strict';
import test from 'node:test';
import { newestPage, StatusIndex, type TrackOutcome } from '../client/src/scripts/statusindex.ts';

test('each file occupies one counter as it passes through provider fallbacks', () => {
    const index = new StatusIndex();
    const record = (path: string, status: TrackOutcome) => index.record({ status: { path, status } });
    record('A', 'error');
    record('B', 'skipped');
    record('A', 'deferred');
    record('A', 'partial');
    record('A', 'error');
    assert.deepEqual(index.counts, { ok: 0, partial: 1, deferred: 0, error: 0, skipped: 1 });
    record('A', 'ok');
    record('A', 'deferred');
    assert.deepEqual(index.counts, { ok: 1, partial: 0, deferred: 0, error: 0, skipped: 1 });
    assert.equal(index.rows[0].attempts.length, 6);
});

test('large runs page newest-first without copying or reversing the underlying history', () => {
    const index = new StatusIndex();
    for (let i = 0; i < 40000; i++) index.record({ status: { path: String(i), status: 'ok' } });
    assert.equal(index.counts.ok, 40000);
    assert.deepEqual(newestPage(index.rows, 1, 2).map(row => row.path), ['39998', '39997']);
    assert.deepEqual(newestPage(index.rows, 39999, 20).map(row => row.path), ['0']);
    assert.deepEqual(newestPage(index.rows, 40000, 20), []);
    assert.equal(index.rows[0].path, '0');
});
