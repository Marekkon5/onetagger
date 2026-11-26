const WINDOW_MS = 60_000;
const MIN_SAMPLE_MS = 5_000;

export class ProviderPace {
    readonly name: string;
    buckets: { second: number; count: number }[] = [];
    firstStartedAt?: number;
    completed = 0;
    limited = 0;

    constructor(name: string) {
        this.name = name;
    }

    record(now: number, lookupMs: number | undefined, limited: boolean): void {
        if (limited) this.limited++;
        if (lookupMs === undefined || !Number.isFinite(lookupMs) || lookupMs < 0) return;

        const startedAt = Math.max(0, now - lookupMs);
        this.firstStartedAt = Math.min(this.firstStartedAt ?? startedAt, startedAt);
        this.completed++;

        const second = Math.floor(now / 1000);
        const last = this.buckets.at(-1);
        if (last?.second === second) {
            last.count++;
        } else {
            this.buckets.push({ second, count: 1 });
        }
        const expired = second - WINDOW_MS / 1000;
        while (this.buckets.length && this.buckets[0].second <= expired) {
            this.buckets.shift();
        }
    }

    perMinute(now: number): number | undefined {
        if (this.firstStartedAt === undefined || this.completed < 3) return undefined;
        const elapsed = Math.min(WINDOW_MS, now - this.firstStartedAt);
        if (elapsed < MIN_SAMPLE_MS) return undefined;

        const expired = Math.floor((now - WINDOW_MS) / 1000);
        const count = this.buckets.reduce((sum, slot) => slot.second > expired ? sum + slot.count : sum, 0);
        return count * WINDOW_MS / elapsed;
    }
}
