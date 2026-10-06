import { spawn } from 'node:child_process';
import type { ImportLimits } from './control.js';

export class MediaRejectedError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'MediaRejectedError';
  }
}

export class FFmpegTimeoutError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'FFmpegTimeoutError';
  }
}

export interface ProbeResult {
  durationMs: number;
  format: string;
  codec?: string;
}

function runBounded(command: string, args: string[], timeoutMs: number): Promise<{ code: number; stdout: string; stderr: string }> {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: ['ignore', 'pipe', 'pipe'], shell: false });
    let stdout = '';
    let stderr = '';
    let settled = false;
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      child.kill('SIGKILL');
      reject(new FFmpegTimeoutError(`${command} exceeded ${timeoutMs}ms`));
    }, timeoutMs);
    child.stdout.on('data', (chunk: Buffer) => { if (stdout.length < 1_000_000) stdout += chunk.toString(); });
    child.stderr.on('data', (chunk: Buffer) => { if (stderr.length < 1_000_000) stderr += chunk.toString(); });
    child.on('error', (error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      reject(error);
    });
    child.on('close', (code) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve({ code: code ?? -1, stdout, stderr });
    });
  });
}

/** Read duration/format from a local file. Never accepts a remote URL. */
export async function boundedFFprobe(filePath: string, limits: ImportLimits): Promise<ProbeResult> {
  if (filePath.includes('://')) throw new MediaRejectedError('ffprobe only accepts local files');
  const { code, stdout, stderr } = await runBounded('ffprobe', [
    '-v', 'error',
    '-print_format', 'json',
    '-show_format',
    '-show_streams',
    '-analyzeduration', '10000000',
    filePath,
  ], 15_000);
  if (code !== 0) throw new MediaRejectedError(`ffprobe failed: ${stderr.slice(0, 500)}`);
  let parsed: { format?: { duration?: string; format_name?: string }; streams?: Array<{ codec_type?: string; codec_name?: string }> };
  try {
    parsed = JSON.parse(stdout);
  } catch {
    throw new MediaRejectedError('ffprobe produced unreadable output');
  }
  const durationSeconds = Number(parsed.format?.duration ?? '0');
  if (!Number.isFinite(durationSeconds) || durationSeconds <= 0) {
    throw new MediaRejectedError('Media has no readable duration');
  }
  const durationMs = Math.round(durationSeconds * 1000);
  if (durationMs > limits.maxDurationMs) {
    throw new MediaRejectedError(`Media duration ${durationMs}ms exceeds the ${limits.maxDurationMs}ms limit`);
  }
  const audio = parsed.streams?.find((stream) => stream.codec_type === 'audio');
  if (!audio) throw new MediaRejectedError('Media has no audio stream');
  return { durationMs, format: parsed.format?.format_name ?? 'unknown', codec: audio.codec_name };
}

/**
 * Normalize a local file to MP3 with ID3v2.3 tags written by ffmpeg itself.
 * Arguments are built from validated config only: no shell, no remote URLs,
 * bounded output size and a hard runtime deadline.
 */
export async function boundedFFmpegConvert(options: {
  inputPath: string;
  outputPath: string;
  bitrateKbps: number;
  maxOutputBytes: number;
  timeoutMs: number;
  metadata?: { title?: string; artist?: string; album?: string; date?: string };
  coverPath?: string;
}): Promise<void> {
  if (options.inputPath.includes('://') || options.outputPath.includes('://')) {
    throw new MediaRejectedError('ffmpeg only accepts local files');
  }
  const args = ['-hide_banner', '-nostdin', '-y'];
  if (options.coverPath) {
    args.push(
      '-i', options.inputPath,
      '-i', options.coverPath,
      '-map', '0:a',
      '-map', '1:v',
      '-c:v', 'mjpeg',
      '-disposition:v', 'attached_pic',
    );
  } else {
    args.push('-i', options.inputPath, '-vn');
  }
  args.push('-c:a', 'libmp3lame', '-b:a', `${options.bitrateKbps}k`, '-id3v2_version', '3');
  for (const [key, value] of Object.entries(options.metadata ?? {})) {
    if (value) args.push('-metadata', `${key}=${value}`);
  }
  args.push('-fs', String(options.maxOutputBytes), options.outputPath);
  const { code, stderr } = await runBounded('ffmpeg', args, options.timeoutMs);
  if (code !== 0) throw new MediaRejectedError(`ffmpeg failed: ${stderr.slice(0, 500)}`);
}

/** Rough wall-clock budget for a conversion: 3x realtime with hard bounds. */
export function conversionTimeoutMs(durationMs: number): number {
  return Math.min(Math.max(durationMs * 3, 60_000), 30 * 60_000);
}
