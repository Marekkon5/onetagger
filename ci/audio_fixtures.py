"""Generate tiny, synthetic codec fixtures on GitHub; no real music is used."""
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / 'crates/onetagger-player/test-fixtures'
FORMATS = {
    'tone.wav': ['-c:a', 'pcm_s16le'],
    'tone.aiff': ['-c:a', 'pcm_s16be'],
    'tone.flac': ['-c:a', 'flac'],
    'tone.mp3': ['-c:a', 'libmp3lame', '-b:a', '128k'],
    'tone.ogg': ['-c:a', 'libvorbis', '-q:a', '3'],
    'tone-aac.m4a': ['-c:a', 'aac', '-b:a', '96k'],
    'tone-alac.m4a': ['-c:a', 'alac'],
}


def main():
    OUTPUT.mkdir(exist_ok=True)
    files = {}
    for name, options in FORMATS.items():
        path = OUTPUT / name
        subprocess.run(['ffmpeg', '-hide_banner', '-loglevel', 'error', '-y', '-f', 'lavfi',
                        '-i', 'sine=frequency=440:sample_rate=44100:duration=2', '-ac', '2',
                        '-map_metadata', '-1', *options, str(path)], check=True)
        data = path.read_bytes()
        files[name] = {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}
    version = subprocess.check_output(['ffmpeg', '-version'], text=True).splitlines()[0]
    (OUTPUT / 'manifest.json').write_text(json.dumps({'generator': version,
        'source': 'Synthetic 440 Hz sine, 44100 Hz, stereo, two seconds', 'files': files}, indent=2) + '\n')


if __name__ == '__main__':
    main()
