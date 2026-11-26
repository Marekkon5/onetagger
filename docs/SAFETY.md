# File operations and browser sessions

## Tag writes

Tag updates for MP3, AIFF, WAV, FLAC, MP4 and Vorbis tags use a private temporary file beside the original. OneTagger copies the original, applies the update, checks write/flush errors, synchronizes the staged file, and publishes it with replacement semantics. A failure before publication leaves the original unchanged and removes the temporary file.

This requires enough free space for a second copy of the audio file. Symlinks are followed to their target; an atomic replacement creates a new filesystem object, so other hard links to the old object are not updated. Basic file permissions are copied. Ownership, ACLs, extended attributes, and filesystem timestamps are not guaranteed to survive replacement. This is protection against partial application writes, not a substitute for backups or a guarantee against every filesystem or power-loss failure.

A missing ID3 tag may be created. A parsing, unsupported-format, or I/O error is returned instead of being treated as an empty tag. WAV input must have a valid, non-truncated RIFF/WAVE header.

The tag editor uses the same missing-ID3 handling when opening and saving a file. Save acknowledgements identify the request and file. Pending edits stay in the editor until a matching success response; a write error or disconnected session keeps them available. The editor pauses interaction during a save. Autosave waits for confirmation before switching tracks or pages; without autosave, discarding dirty edits requires confirmation. These pending edits live in browser memory and do not survive a page reload or application exit.

Album tagging processes only the files supplied to the batch, including playlist subsets. Other files in the same folder are not implicitly added. Provider failures are deduplicated per file, and any successful provider removes that file from final failed-file processing.

Downloaded artwork is decoded before existing covers are removed. Unavailable or invalid replacement artwork leaves the original covers intact; other requested metadata can still be saved. Non-JPEG downloads are converted to JPEG to match the embedded MIME and sidecar extension.

## Moves and renames

The renamer checks the full planned batch for duplicate destinations and destinations that are another source in the batch. Such conflicts are reported without modifying those entries, including with overwrite enabled. Existing destinations are rejected unless overwrite was selected. Identical source and destination files are left unchanged. Moving a symbolic link is rejected without touching its target; select the target file explicitly instead. Copying through a symbolic link copies the target data.

Every entry reports moved, copied, unchanged, conflict, or failed. The GUI shows counts and errors; the CLI exits with status 1 when any operation failed or conflicted. Successful independent entries remain completed, so a batch is not an all-or-nothing transaction. If a copy completes but deleting its source fails, both may remain and the error reports that situation.

AutoTagger's move-success/move-failed operations never silently accept a conflicting destination. Result playlists retain the actual source path when a move fails, and the GUI reports move errors separately from tag results. The CLI also exits with status 1 if a move fails.

## Browser sessions

The server creates a random 256-bit capability on each process start. The desktop window and `--browser` receive it in the URL fragment, which is not sent in HTTP requests. The frontend removes the fragment before starting the router and exchanges it for an HttpOnly, SameSite=Strict cookie.

WebSocket commands and the audio/thumbnail endpoints require the session. WebSocket upgrades also require an Origin matching the serving authority. Loopback mode rejects unexpected Host headers. Restarting invalidates earlier sessions. Session capabilities are excluded from serialized startup data, debug output, and the downloadable application log. Do not share launch links or terminal captures containing them.

`--expose` allows authenticated clients to use the application's filesystem features with the server account's permissions. For a remote client, replace `127.0.0.1` in the printed launch link with the server or proxy address, keeping its fragment private. For access beyond a trusted local machine, use an HTTPS reverse proxy that preserves the intended Host and Origin. There is no per-user authorization model or sandbox for authenticated sessions; keep launch links private. Native plugins also run with the application's permissions.

The static frontend and Spotify OAuth callback remain public routes. The callback is needed before a Spotify login completes; the session capability must never be placed in the OAuth redirect URL.

WebSocket messages and frames are capped at 16 MiB. Very large embedded artwork may need to be reduced before editing.
