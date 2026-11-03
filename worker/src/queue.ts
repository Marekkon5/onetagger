import { ControlClient } from './control.js';
import { FFmpegTimeoutError, MediaRejectedError } from './ffmpeg.js';
import { NonRetryableError } from './resolvers.js';
import { RetryableError, processAudioJob, processMetadataJob } from './process.js';
import { UpstreamHttpError, UrlBlockedError } from './security.js';
import type { WorkerConfig } from './config.js';

export function classifyError(error: unknown): { retryable: boolean; errorCode: string; message: string } {
  if (error instanceof UpstreamHttpError) {
    // Provider throttling and server errors are transient; auth/not-found are not.
    const retryable = error.status === 408 || error.status === 425 || error.status === 429 || error.status >= 500;
    return {
      retryable,
      errorCode: retryable ? 'upstream_transient' : 'upstream_rejected',
      message: error.message,
    };
  }
  if (error instanceof NonRetryableError || error instanceof UrlBlockedError || error instanceof MediaRejectedError) {
    return { retryable: false, errorCode: 'non_retryable', message: String((error as Error).message) };
  }
  if (error instanceof RetryableError) {
    return { retryable: true, errorCode: 'retryable', message: String(error.message) };
  }
  if (error instanceof FFmpegTimeoutError) {
    return { retryable: true, errorCode: 'timeout', message: String(error.message) };
  }
  const message = error instanceof Error ? error.message : String(error);
  if (/timeout|timed out|ECONNRESET|EAI_AGAIN|429|50[0-3]|lease lost/i.test(message)) {
    return { retryable: true, errorCode: 'transient', message };
  }
  return { retryable: true, errorCode: 'unknown', message };
}

/** Single poll iteration. Returns true when a job was handled. */
export async function pollOnce(client: ControlClient, config: WorkerConfig): Promise<boolean> {
  const spec = await client.claimNext();
  if (!spec) return false;
  const { job } = spec;
  log(client, job);
  try {
    if (job.mode === 'audio') await processAudioJob(client, config, spec);
    else await processMetadataJob(client, config, spec);
    await client.complete(job.id, { state: 'completed', attempt: job.attempt });
  } catch (error) {
    const classified = classifyError(error);
    try {
      await client.event(job.id, 'failed', { error: classified.message, retryable: classified.retryable });
      await client.complete(job.id, {
        state: 'failed',
        attempt: job.attempt,
        error: classified.message,
        errorCode: classified.errorCode,
        retryable: classified.retryable,
      });
    } catch (reportError) {
      console.error(`job=${job.id} could not report failure`, reportError);
    }
  }
  return true;
}

function log(_client: ControlClient, job: { id: string; provider: string; mode: string; attempt: number }) {
  console.log(JSON.stringify({
    level: 'info', job_id: job.id, attempt: job.attempt,
    provider: job.provider, stage: 'claimed', mode: job.mode,
  }));
}
