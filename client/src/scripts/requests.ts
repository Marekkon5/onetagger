/** Correlate replies with both their request and file; never replay writes. */
export class Requests<T> {
    private pending = new Map<string, { path: string; resolve: (value: T) => void;
        reject: (error: Error) => void; timer: ReturnType<typeof setTimeout> }>();

    run(id: string, path: string, send: () => Promise<void>, timeout = 30000): Promise<T> {
        return new Promise<T>((resolve, reject) => {
            const timer = setTimeout(() => this.finish(id, path, undefined, new Error('Save acknowledgement timed out')), timeout);
            this.pending.set(id, { path, resolve, reject, timer });
            send().catch(error => this.finish(id, path, undefined, error instanceof Error ? error : new Error(String(error))));
        });
    }

    has(id: string, path: string): boolean { return this.pending.get(id)?.path === path; }

    finish(id: string, path: string, value?: T, error?: Error): boolean {
        const request = this.pending.get(id);
        if (!request || request.path !== path) return false;
        clearTimeout(request.timer);
        this.pending.delete(id);
        if (error) request.reject(error);
        else request.resolve(value as T);
        return true;
    }

    disconnect(error: Error): void {
        for (const [id, request] of this.pending) this.finish(id, request.path, undefined, error);
    }
}
