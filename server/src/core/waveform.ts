import { spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { once } from 'node:events';
import { promisify } from 'node:util';
import { execFile } from 'node:child_process';

const MAX_AUDIO_BYTES = 250 * 1024 * 1024;
const MAX_WAVEFORM_BYTES = 1024 * 1024;
const TIMEOUT_MS = 5 * 60_000;

export interface WaveformJob {
  id: string;
  sourceUrl: string;
  state: 'queued' | 'running' | 'done' | 'error';
  progress: number;
  imageDataUrl?: string;
  error?: string;
  analyzedAt?: string;
}

function allowedAudioUrl(value: string): URL {
  let url: URL;
  try { url = new URL(value); } catch { throw new Error('A public audio URL is required'); }
  const host = url.hostname.toLowerCase();
  const trustedHost = host === 'freeteknomusic.org' || host === 'www.freeteknomusic.org'
    || host === 'archive.freeteknomusic.org' || host === 'archive.org' || host.endsWith('.archive.org');
  if (url.protocol !== 'https:' || url.username || url.password || url.port || !trustedHost
    || !/\.(mp3|flac|ogg|oga|wav|m4a|aac|aif|aiff)$/i.test(url.pathname)) {
    throw new Error('Waveform analysis supports direct public audio from Freeteknomusic or Archive.org');
  }
  return url;
}

const execFileAsync = promisify(execFile);

export async function probeAudioDuration(sourceUrl: string, signal?: AbortSignal): Promise<number | undefined> {
  const url = allowedAudioUrl(sourceUrl);
  const { stdout } = await execFileAsync('ffprobe', [
    '-v', 'error', '-show_entries', 'format=duration', '-of', 'default=noprint_wrappers=1:nokey=1', url.toString(),
  ], { timeout: 15_000, maxBuffer: 4096, signal });
  const seconds = Number(stdout.trim());
  return Number.isFinite(seconds) && seconds > 0 ? Math.round(seconds * 1000) : undefined;
}

async function fetchAudio(value: string, signal: AbortSignal): Promise<Response> {
  let url = allowedAudioUrl(value);
  for (let redirect = 0; redirect < 4; redirect += 1) {
    const response = await fetch(url, { signal, redirect: 'manual', headers: { accept: 'audio/*,application/octet-stream;q=0.8' } });
    if ([301, 302, 303, 307, 308].includes(response.status)) {
      const location = response.headers.get('location');
      if (!location) throw new Error('Audio redirect had no destination');
      url = allowedAudioUrl(new URL(location, url).toString());
      continue;
    }
    if (!response.ok || !response.body) throw new Error(`Audio source returned HTTP ${response.status}`);
    const contentLength = Number(response.headers.get('content-length') || 0);
    if (contentLength > MAX_AUDIO_BYTES) throw new Error('Audio exceeds the 250 MiB analysis limit');
    return response;
  }
  throw new Error('Audio source redirected too many times');
}

async function analyze(job: WaveformJob): Promise<void> {
  job.state = 'running';
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(new Error('Audio analysis timed out')), TIMEOUT_MS);
  let child: ReturnType<typeof spawn> | undefined;
  try {
    const response = await fetchAudio(job.sourceUrl, controller.signal);
    const audioBody = response.body!;
    const expected = Number(response.headers.get('content-length') || 0);
    child = spawn('ffmpeg', [
      '-hide_banner', '-loglevel', 'error', '-nostdin', '-i', 'pipe:0',
      '-filter_complex', 'aformat=channel_layouts=mono,showwavespic=s=960x120:colors=0xD8FF30',
      '-frames:v', '1', '-f', 'image2pipe', '-vcodec', 'png', 'pipe:1',
    ], { stdio: ['pipe', 'pipe', 'pipe'] });
    const process = child;
    const input = process.stdin!;
    const stdout = process.stdout!;
    const stderr = process.stderr!;
    const output: Buffer[] = [];
    let outputBytes = 0;
    let errorText = '';
    stdout.on('data', (chunk: Buffer) => {
      outputBytes += chunk.length;
      if (outputBytes > MAX_WAVEFORM_BYTES) process.kill('SIGKILL');
      else output.push(chunk);
    });
    stderr.on('data', (chunk: Buffer) => { errorText = (errorText + chunk.toString()).slice(-600); });
    const exited = new Promise<number>((resolve, reject) => {
      process.once('error', reject);
      process.once('close', (code) => resolve(code ?? -1));
    });
    controller.signal.addEventListener('abort', () => process.kill('SIGKILL'), { once: true });
    let received = 0;
    for await (const chunk of audioBody) {
      if (controller.signal.aborted) throw controller.signal.reason;
      received += chunk.length;
      if (received > MAX_AUDIO_BYTES) throw new Error('Audio exceeds the 250 MiB analysis limit');
      if (!input.write(chunk)) await once(input, 'drain');
      if (expected) job.progress = Math.min(95, Math.round(received / expected * 95));
    }
    input.end();
    const exitCode = await exited;
    if (controller.signal.aborted) throw controller.signal.reason;
    if (exitCode !== 0 || !outputBytes) throw new Error(errorText || 'Audio could not be decoded');
    job.imageDataUrl = `data:image/png;base64,${Buffer.concat(output).toString('base64')}`;
    job.analyzedAt = new Date().toISOString();
    job.progress = 100;
    job.state = 'done';
  } catch (error) {
    child?.kill('SIGKILL');
    job.error = error instanceof Error ? error.message : String(error);
    job.state = 'error';
  } finally {
    clearTimeout(timeout);
  }
}

export class WaveformQueue {
  private readonly jobs = new Map<string, WaveformJob>();

  create(sourceUrl: string): WaveformJob {
    allowedAudioUrl(sourceUrl);
    if ([...this.jobs.values()].some((job) => job.state === 'running' || job.state === 'queued')) {
      throw new Error('An audio analysis is already running; retry when it finishes');
    }
    const job: WaveformJob = { id: randomUUID(), sourceUrl, state: 'queued', progress: 0 };
    this.jobs.set(job.id, job);
    if (this.jobs.size > 16) {
      const oldest = this.jobs.keys().next().value;
      if (oldest) this.jobs.delete(oldest);
    }
    void analyze(job);
    return job;
  }

  get(id: string): WaveformJob | undefined { return this.jobs.get(id); }
}
