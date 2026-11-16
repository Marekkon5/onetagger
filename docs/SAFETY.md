# File operations and browser sessions

## Tag writes

Tag updates for MP3, AIFF, WAV, FLAC, MP4 and Vorbis tags use a private temporary file beside the original. OneTagger copies the original, applies the update, checks write/flush errors, synchronizes the staged file, and publishes it with replacement semantics. A failure before publication leaves the original unchanged and removes the temporary file.

This requires enough free space for a second copy of the audio file. Symlinks are followed to their target; an atomic replacement creates a new filesystem object, so other hard links to the old object are not updated. File permissions are preserved. This is protection against partial application writes, not a substitute for backups or a guarantee against every filesystem or power-loss failure.

A missing ID3 tag may be created. A parsing, unsupported-format, or I/O error is returned instead of being treated as an empty tag. WAV input must have a valid, non-truncated RIFF/WAVE header.

## Moves and renames

The renamer checks the full planned batch for duplicate destinations and destinations that are another source in the batch. Such conflicts are reported without modifying those entries, including with overwrite enabled. Existing destinations are rejected unless overwrite was selected. Identical source and destination files are left unchanged. Moving a symbolic link is rejected without touching its target; select the target file explicitly instead. Copying through a symbolic link copies the target data.

Every entry reports moved, copied, unchanged, conflict, or failed. The GUI shows counts and errors; the CLI exits with status 1 when any operation failed or conflicted. Successful independent entries remain completed, so a batch is not an all-or-nothing transaction. If a copy completes but deleting its source fails, both may remain and the error reports that situation.

AutoTagger's move-success/move-failed operations never silently accept a conflicting destination. Result playlists retain the actual source path when a move fails, and the GUI reports move errors separately from tag results. The CLI also exits with status 1 if a move fails.

## Browser sessions

The server creates a random 256-bit capability on each process start. The desktop window and `--browser` receive it in the URL fragment, which is not sent in HTTP requests. The frontend removes the fragment before starting the router and exchanges it for an HttpOnly, SameSite=Strict cookie.

WebSocket commands and the audio/thumbnail endpoints require the session. WebSocket upgrades also require an Origin matching the serving authority. Loopback mode rejects unexpected Host headers. Restarting invalidates earlier sessions. Session capabilities are excluded from serialized startup data, debug output, and the downloadable application log. Do not share launch links or terminal captures containing them.

`--expose` allows authenticated clients to use the application's filesystem features with the server account's permissions. For access beyond a trusted local machine, use an HTTPS reverse proxy that preserves the intended Host and Origin. There is no per-user authorization model or sandbox for authenticated sessions; keep launch links private. Native plugins also run with the application's permissions.

The static frontend and Spotify OAuth callback remain public routes. The callback is needed before a Spotify login completes; the session capability must never be placed in the OAuth redirect URL.

WebSocket messages and frames are capped at 16 MiB. Very large embedded artwork may need to be reduced before editing.
