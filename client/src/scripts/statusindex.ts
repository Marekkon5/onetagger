export type TrackOutcome = 'ok' | 'partial' | 'deferred' | 'error' | 'skipped';
const rank: Record<TrackOutcome, number> = { ok: 5, partial: 4, deferred: 3, error: 2, skipped: 1 };

export interface StatusGroup<T> {
    path: string;
    state: TrackOutcome;
    attempts: T[];
}

export class StatusIndex<T extends { status: { path: string; status: TrackOutcome } }> {
    rows: StatusGroup<T>[] = [];
    readonly byPath = new Map<string, StatusGroup<T>>();
    counts: Record<TrackOutcome, number> = { ok: 0, partial: 0, deferred: 0, error: 0, skipped: 0 };

    record(attempt: T) {
        const { path, status } = attempt.status;
        const row = this.byPath.get(path);
        if (!row) {
            const group = { path, state: status, attempts: [attempt] };
            this.rows.push(group);
            this.byPath.set(path, group);
            this.counts[status]++;
            return;
        }
        row.attempts.push(attempt);
        if (rank[status] > rank[row.state]) {
            this.counts[row.state]--;
            this.counts[status]++;
            row.state = status;
        }
    }
}

// The virtual list requests only the visible slice, newest first.
export function newestPage<T>(rows: T[], from: number, size: number): T[] {
    const end = Math.max(0, rows.length - from);
    return rows.slice(Math.max(0, end - size), end).reverse();
}
