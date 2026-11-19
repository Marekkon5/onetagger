"""Produce initial drafts for human review. This is a read-only, temporary CI utility."""
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import re
import sys
import tempfile
import urllib.request
import unicodedata

import ctranslate2
from huggingface_hub import HfApi, snapshot_download
from transformers import AutoTokenizer


TOKENS = re.compile(r"\{(?:[a-zA-Z_][a-zA-Z0-9_]*|'(?:\\.|[^'\\])*')\}|%[a-zA-Z_]+%|\$(?:success|failed)")
SLOT_MARKERS = tuple(str(9876543100 + i) for i in range(26))
EXACT_TERMS = {
    "Home", "Settings", "Preferences", "Cancel", "OK", "Save", "Close", "Add", "Remove", "Delete",
    "Play", "Pause", "Duration", "Track number", "Title", "Artist", "Album", "Year", "Genre",
    "Composer", "Comments", "Path", "Name", "Description", "Language", "Volume", "Rating", "Colour",
    "BPM", "ISRC", "Copyright", "Search tracks", "Parent folder", "Metadata", "Playlist", "Warning",
}


def prepare(segment):
    replacements = []

    def hide(match):
        marker = SLOT_MARKERS[len(replacements)]
        replacements.append((marker, match[0]))
        return marker

    return TOKENS.sub(hide, segment).strip(), replacements


def restore(text, replacements):

    if replacements:
        text = ''.join(str(unicodedata.decimal(ch)) if ch.isdecimal() else ch for ch in text)
    for token, placeholder in replacements:
        spaced = r"[\s,.،٬]*".join(token)
        text = re.sub(spaced, lambda _: placeholder, text)
    return text


def main():
    locale = sys.argv[1]
    repo = "facebook/m2m100_418M"
    info = HfApi().model_info(repo)
    if info.card_data is None or info.card_data.license != "mit":
        raise ValueError(f"Unexpected model license for {repo}")
    print(f"Draft model {repo} at {info.sha}", flush=True)
    ref = os.environ["GITHUB_HELPER_REF"]
    url = f"https://raw.githubusercontent.com/fantuz/rekordbox-migration-helper/{ref}/static/locales.json"
    with ThreadPoolExecutor(max_workers=2) as downloader:
        model_files = downloader.submit(snapshot_download, repo, revision=info.sha,
                                        allow_patterns=["*.json", "*.spm", "*.model", "model.safetensors", "pytorch_model.bin"])
        with urllib.request.urlopen(url, timeout=45) as response:
            helper = json.load(response)["messages"]
        model_path = model_files.result()

    tokenizer = AutoTokenizer.from_pretrained(model_path, local_files_only=True, trust_remote_code=False)
    references = {p.stem: json.loads(p.read_text(encoding="utf-8"))
                  for p in sorted(Path("client/src/locales/en").glob("*.json"))}
    segments = dict.fromkeys(
        segment.strip()
        for catalog in references.values()
        for source in catalog.values()
        for segment in re.split(r"(\s*\|\s*|\n+)", source)[::2]
        if segment.strip()
    )
    translated = {}
    to_infer = []
    for source in segments:
        existing = helper.get(source, {}).get(locale) if source in EXACT_TERMS else None
        if isinstance(existing, str) and existing.strip():
            translated[source] = existing
        elif not re.search(r"[A-Za-z]", source):
            translated[source] = source
        else:
            to_infer.append(source)
    slots = {source: prepare(source) for source in to_infer}
    with tempfile.TemporaryDirectory() as temp:
        converted = ctranslate2.converters.TransformersConverter(model_path, trust_remote_code=False)
        converted.convert(temp, quantization="int8", force=True)
        translator = ctranslate2.Translator(temp, compute_type="int8", intra_threads=4)
        target = [tokenizer.lang_code_to_token[locale]]
        for start in range(0, len(to_infer), 96):
            batch = to_infer[start:start + 96]
            source_ids = tokenizer([slots[item][0] for item in batch],
                                   truncation=True, max_length=490).input_ids
            tokens = [tokenizer.convert_ids_to_tokens(ids) for ids in source_ids]
            predictions = translator.translate_batch(tokens, target_prefix=[target] * len(batch),
                                                    beam_size=3, max_batch_size=1024, batch_type="tokens",
                                                    max_decoding_length=350, repetition_penalty=1.05)
            for source, prediction in zip(batch, predictions):
                ids = tokenizer.convert_tokens_to_ids(prediction.hypotheses[0][1:])
                proposal = tokenizer.decode(ids, skip_special_tokens=True, clean_up_tokenization_spaces=False).strip()
                translated[source] = restore(proposal, slots[source][1])
            print(f"Translated {min(start+96, len(to_infer))}/{len(to_infer)} draft segments", flush=True)
    for namespace, catalog in references.items():
        draft = {}
        needs_review = []
        for key, source in catalog.items():
            pieces = re.split(r"(\s*\|\s*|\n+)", source)
            result = "".join(translated.get(segment.strip(), segment) if i % 2 == 0 else segment
                             for i, segment in enumerate(pieces))
            if not result.strip() or set(TOKENS.findall(result)) != set(TOKENS.findall(source)) or "98765431" in result:
                needs_review.append(key)
            draft[key] = result
        print("ONETAGGER_DRAFT " + namespace + " " + json.dumps(draft, ensure_ascii=False), flush=True)
        print("ONETAGGER_REVIEW " + namespace + " " + json.dumps(needs_review), flush=True)


if __name__ == "__main__":
    main()
