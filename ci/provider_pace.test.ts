import assert from 'node:assert/strict';
import test from 'node:test';
import { ProviderPace } from '../client/src/scripts/providerpace.ts';

test('provider pace uses a sustained completed-attempt sample', () => {
    const pace = new ProviderPace('fixture provider');
    pace.record(2000, 2000, false);
    pace.record(4000, 2000, false);
    assert.equal(pace.perMinute(4000), undefined);
    for (let now = 6000; now <= 12000; now += 2000) {
        pace.record(now, 2000, false);
    }
    assert.equal(pace.perMinute(12000), 30);
});

test('deferred work is counted separately and idle time lowers the rate', () => {
    const pace = new ProviderPace('fixture provider');
    pace.record(2000, 2000, false);
    pace.record(4000, 2000, false);
    pace.record(6000, 2000, true);
    pace.record(8000, undefined, true);
    assert.equal(pace.completed, 3);
    assert.equal(pace.limited, 2);
    assert.equal(pace.perMinute(12000), 15);
    assert.equal(pace.perMinute(66000), 0);
});

test('the last minute is kept in at most sixty buckets', () => {
    const pace = new ProviderPace('fixture provider');
    for (let now = 1000; now <= 150000; now += 1000) {
        pace.record(now, 30, false);
        pace.record(now + 50, 30, false);
    }
    assert.equal(pace.buckets.length, 60);
    assert.equal(pace.perMinute(150100), 120);
});
