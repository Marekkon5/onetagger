import { spawn } from 'node:child_process';
import { unlink } from 'node:fs/promises';
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
  /** Actual audio bitrate in kbps, measured from the stream or packet sizes. Absent when unmeasurable. */
  bitrateKbps?: number;
  sampleRate?: number;
  channels?: number;
  /** True when bitrateKbps came from the audio stream or packet sum, not the container. */
  bitrateMeasured: boolean;
}

/** Fixed CBR values supported by MP3. Never round a source's measured kbps up. */
export const MP3_CBR_RATES_K: readonly number[] = [8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320];
const MP3_SAMPLE_RATES_HZ: readonly number[] = [8000, 11025, 12000, 16000, 22050, 24000, 32000, 44100, 48000];

function edgeRate(rates: readonly number[], limit: number): number | undefined {
  for (let i = rates.length - 1; i >= 0; i--) {
    if (rates[i]! <= limit) return rates[i];
  }
  return undefined;
}

/**
 * Highest valid CBR bitrate not exceeding either the requested ceiling or the
 * measured source. A missing/unusable bitrate is not permission to produce a
 * 320-kbit/s MP3: fail closed so the caller can reject the media instead of
 * silently upsampling (sc-suite audio-generation policy).
 */
export function guardedBitrateKbps(requestedKbps: number, sourceKbps: number | undefined): number {
  if (sourceKbps === undefined || !Number.isFinite(sourceKbps) || sourceKbps <= 0) {
    throw new MediaRejectedError('Source audio bitrate unavailable');
  }
  const rate = edgeRate(MP3_CBR_RATES_K, Math.min(requestedKbps, sourceKbps));
  if (rate === undefined) {
    throw new MediaRejectedError(`Source bitrate ${sourceKbps} kbps is below supported MP3 CBR rates`);
  }
  return rate;
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

/**
 * Sum audio packet sizes without buffering the whole stream listing: long
 * recordings can exceed a string cap, but the running total never grows.
 */
async function sumAudioPacketBytes(filePath: string, timeoutMs: number): Promise<number | undefined> {
  return new Promise((resolve) => {
    const child = spawn('ffprobe', [
      '-v', 'error',
      '-select_streams', 'a:0',
      '-show_packets',
      '-show_entries', 'packet=size',
      '-of', 'csv=p=0',
      filePath,
    ], { stdio: ['ignore', 'pipe', 'pipe'], shell: false });
    let totalBytes = 0;
    let remainder = '';
    let settled = false;
    const finish = (result: number | undefined) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve(result);
    };
    const timer = setTimeout(() => {
      child.kill('SIGKILL');
      finish(undefined);
    }, timeoutMs);
    child.stdout.on('data', (chunk: Buffer) => {
      const text = remainder + chunk.toString();
      const lines = text.split('\n');
      remainder = lines.pop() ?? '';
      for (const line of lines) {
        const size = Number.parseInt(line.trim(), 10);
        if (!Number.isFinite(size) || size < 0) finish(undefined);
        else totalBytes += size;
      }
    });
    child.on('error', () => finish(undefined));
    child.on('close', () => {
      if (remainder.trim().length > 0) {
        const size = Number.parseInt(remainder.trim(), 10);
        if (!Number.isFinite(size) || size < 0) { finish(undefined); return; }
        totalBytes += size;
      }
      finish(totalBytes > 0 ? totalBytes : undefined);
    });
  });
}

interface RawProbe {
  format?: { duration?: string; format_name?: string };
  streams?: Array<{ codec_type?: string; codec_name?: string; bit_rate?: string; sample_rate?: string; channels?: number }>;
}

export function parseAudioProbe(data: RawProbe): Omit<ProbeResult, 'durationMs'> {
  const streams = data.streams ?? [];
  const audio = streams.find((stream) => stream.codec_type === 'audio');
  if (!audio) throw new MediaRejectedError('Media has no audio stream');
  const streamBitrate = Number(audio.bit_rate);
  const bitrateMeasured = Number.isFinite(streamBitrate) && streamBitrate > 0;
  // Container overhead must never become the encode cap: when the audio
  // stream omits bit_rate (Opus, some VBR), the caller recomputes the bitrate
  // from audio packet sizes instead of trusting format.bit_rate.
  return {
    format: data.format?.format_name ?? 'unknown',
    codec: audio.codec_name,
    ...(bitrateMeasured ? { bitrateKbps: streamBitrate / 1000 } : {}),
    ...(audio.sample_rate ? { sampleRate: Number(audio.sample_rate) } : {}),
    ...(audio.channels ? { channels: audio.channels } : {}),
    bitrateMeasured,
  };
}

/** Read duration/format/bitrate from a local file. Never accepts a remote URL. */
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
  let parsed: RawProbe;
  try {
    parsed = JSON.parse(stdout) as RawProbe;
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
  const probed = parseAudioProbe(parsed);
  if (probed.bitrateKbps === undefined) {
    const totalBytes = await sumAudioPacketBytes(filePath, 15_000);
    if (totalBytes === undefined) {
      throw new MediaRejectedError('Source audio bitrate unavailable: stream has no bit_rate and packet sizes are unreadable');
    }
    probed.bitrateKbps = (totalBytes * 8) / (durationSeconds * 1000);
    probed.bitrateMeasured = true;
  }
  return { durationMs, ...probed };
}

/**
 * Normalize a local file to MP3 following the sc-suite audio-generation
 * policy: stream-copy bit-exact when the source is already MP3, otherwise
 * encode at the highest supported CBR rate not above the requested ceiling or
 * the measured source bitrate (never upsampling), snap unsupported sample
 * rates, downmix beyond stereo, then verify the result by full decode and an
 * output-probe comparison. Arguments are built from validated config only:
 * no shell, no remote URLs, bounded output size and hard runtime deadlines.
 */
export async function boundedFFmpegConvert(options: {
  inputPath: string;
  outputPath: string;
  bitrateKbps: number;
  maxOutputBytes: number;
  timeoutMs: number;
  source: ProbeResult;
  limits: ImportLimits;
  copyWhenSourceIsMp3?: boolean;
  decodeCheck?: boolean;
  metadata?: { title?: string; artist?: string; album?: string; date?: string };
  coverPath?: string;
}): Promise<{ mode: 'copy' | 'encode'; source: ProbeResult; output: ProbeResult; bitrateKbps?: number }> {
  if (options.inputPath.includes('://') || options.outputPath.includes('://')) {
    throw new MediaRejectedError('ffmpeg only accepts local files');
  }

  const copy = options.copyWhenSourceIsMp3 !== false && options.source.codec === 'mp3';
  const encodeArgs: string[] = [];
  let selectedBitrate: number | undefined;
  if (copy) {
    encodeArgs.push('-c:a', 'copy');
  } else {
    selectedBitrate = guardedBitrateKbps(options.bitrateKbps, options.source.bitrateKbps);
    encodeArgs.push('-c:a', 'libmp3lame', '-b:a', `${selectedBitrate}k`);
    if (options.source.sampleRate !== undefined && !MP3_SAMPLE_RATES_HZ.includes(options.source.sampleRate)) {
      encodeArgs.push('-ar', options.source.sampleRate % 48000 === 0 ? '48000' : '44100');
    }
    if (options.source.channels !== undefined && options.source.channels > 2) {
      encodeArgs.push('-ac', '2');
    }
  }

  const args = ['-nostdin', '-v', 'error', '-y'];
  if (options.coverPath) {
    args.push(
      '-i', options.inputPath,
      '-i', options.coverPath,
      '-map', '0:a:0',
      '-map', '1:v:0',
      '-c:v', 'mjpeg',
      '-disposition:v', 'attached_pic',
    );
  } else {
    args.push('-i', options.inputPath, '-vn', '-map', '0:a:0');
  }
  args.push(...encodeArgs, '-map_metadata', '-1', '-id3v2_version', '3');
  for (const [key, value] of Object.entries(options.metadata ?? {})) {
    if (value) args.push('-metadata', `${key}=${value}`);
  }
  args.push('-f', 'mp3', '-fs', String(options.maxOutputBytes), options.outputPath);
  const { code, stderr } = await runBounded('ffmpeg', args, options.timeoutMs);
  if (code !== 0) throw new MediaRejectedError(`ffmpeg failed: ${stderr.slice(0, 500)}`);

  if (options.decodeCheck !== false) {
    const checked = await runBounded('ffmpeg', [
      '-nostdin', '-nostats', '-v', 'error', '-i', options.outputPath, '-f', 'null', '-',
    ], options.timeoutMs);
    if (checked.code !== 0 || checked.stderr.trim() !== '') {
      throw new MediaRejectedError(`Decoded output failed verification: ${(checked.stderr || 'ffmpeg exited nonzero').slice(0, 500)}`);
    }
  }

  const output = await boundedFFprobe(options.outputPath, options.limits);
  if (!copy) {
    const sourceKbps = options.source.bitrateKbps;
    if (
      output.bitrateKbps === undefined || sourceKbps === undefined ||
      !Number.isFinite(output.bitrateKbps) || output.bitrateKbps > sourceKbps
    ) {
      await unlink(options.outputPath).catch(() => {});
      throw new MediaRejectedError('Encoded MP3 bitrate exceeds or cannot be compared to the source audio bitrate');
    }
  }
  return { mode: copy ? 'copy' : 'encode', source: options.source, output, bitrateKbps: selectedBitrate };
}

/** Rough wall-clock budget for a conversion: 3x realtime with hard bounds. */
export function conversionTimeoutMs(durationMs: number): number {
  return Math.min(Math.max(durationMs * 3, 60_000), 30 * 60_000);
}
