import assert from 'node:assert/strict';
import test from 'node:test';
import { Requests } from '../client/src/scripts/requests.ts';

test('save acknowledgements require matching request and path and settle once', async () => {
    const requests = new Requests<string>();
    const result = requests.run('save-1', '/music/one.mp3', async () => {});
    assert.equal(requests.finish('save-2', '/music/one.mp3', 'wrong request'), false);
    assert.equal(requests.finish('save-1', '/music/two.mp3', 'wrong file'), false);
    assert.equal(requests.has('save-1', '/music/one.mp3'), true);
    assert.equal(requests.finish('save-1', '/music/one.mp3', 'saved'), true);
    assert.equal(await result, 'saved');
    assert.equal(requests.finish('save-1', '/music/one.mp3', 'duplicate'), false);
});

test('failed sends, explicit failures and missing replies all release pending saves', async () => {
    const requests = new Requests<void>();
    await assert.rejects(requests.run('send', '/one', async () => { throw new Error('Disconnected'); }), /Disconnected/);
    const failed = requests.run('write', '/two', async () => {});
    requests.finish('write', '/two', undefined, new Error('Read only'));
    await assert.rejects(failed, /Read only/);
    await assert.rejects(requests.run('timeout', '/three', async () => {}, 20), /timed out/);
    for (const [id, path] of [['send', '/one'], ['write', '/two'], ['timeout', '/three']]) assert.equal(requests.has(id, path), false);
});

test('disconnect settles every pending write without sending it again', async () => {
    const requests = new Requests<void>();
    let sent = 0;
    const first = requests.run('one', '/one', async () => { sent++; });
    const second = requests.run('two', '/two', async () => { sent++; });
    requests.disconnect(new Error('Connection closed'));
    await Promise.all([assert.rejects(first, /Connection closed/), assert.rejects(second, /Connection closed/)]);
    assert.equal(sent, 2);
    assert.equal(requests.has('one', '/one'), false);
});
