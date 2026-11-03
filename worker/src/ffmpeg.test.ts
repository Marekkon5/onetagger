import assert from 'node:assert/strict';
import test from 'node:test';
import {
  guardedBitrateKbps,
  MP3_CBR_RATES_K,
  parseAudioProbe,
  MediaRejectedError,
} from './ffmpeg.js';

test('guarded bitrate never upsamples and selects the highest legal CBR rate', () => {
  assert.equal(guardedBitrateKbps(320, 320), 320);
  assert.equal(guardedBitrateKbps(320, 321), 320);
  assert.equal(guardedBitrateKbps(320, 130), 128); // 129/130 kbps source becomes 128, not 320
  assert.equal(guardedBitrateKbps(96, 130), 96); // ceiling wins over source
  for (const rate of MP3_CBR_RATES_K) {
    assert.equal(guardedBitrateKbps(320, rate), rate);
  }
});

test('guarded bitrate fails closed on unknown or unusable source bitrate', () => {
  for (const source of [undefined, 0, -1, Number.NaN]) {
    assert.throws(() => guardedBitrateKbps(320, source as number | undefined), MediaRejectedError);
  }
  assert.throws(() => guardedBitrateKbps(320, 7), (error: unknown) => {
    assert.ok(error instanceof MediaRejectedError);
    assert.match(error.message, /below supported MP3 CBR rates/);
    return true;
  });
});

test('probe parsing trusts only the audio stream bitrate, not the container', () => {
  const probed = parseAudioProbe({
    format: { duration: '12.345', format_name: 'mp3' },
    streams: [{ codec_type: 'audio', codec_name: 'mp3', bit_rate: '128000', sample_rate: '44100', channels: 2 }],
  });
  assert.equal(probed.bitrateKbps, 128);
  assert.equal(probed.bitrateMeasured, true);
  assert.equal(probed.sampleRate, 44100);
  assert.equal(probed.channels, 2);
});

test('probe parsing marks opus/omit-bitrate sources as unmeasured for the packet fallback', () => {
  const probed = parseAudioProbe({
    format: { duration: '7200', format_name: 'ogg' },
    streams: [{ codec_type: 'audio', codec_name: 'opus', sample_rate: '48000', channels: 2 }],
  });
  assert.equal(probed.bitrateKbps, undefined);
  assert.equal(probed.bitrateMeasured, false);
  assert.equal(probed.codec, 'opus');
});

test('probe parsing rejects media without an audio stream', () => {
  assert.throws(() => parseAudioProbe({
    format: { duration: '12.345', format_name: 'mp4' },
    streams: [{ codec_type: 'video', codec_name: 'h264' }],
  }), MediaRejectedError);
  assert.throws(() => parseAudioProbe({ format: {} }), MediaRejectedError);
});
