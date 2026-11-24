# Windows tag saves

Quick Tag and Edit Tags use the same atomic tag writer. It updates a private
sibling copy of the audio, syncs the completed file, and replaces the original.
The original must remain intact when tag generation or publication fails.

Streaming playback keeps an audio file open after selection, even while paused.
The `tempfile` 3.27.0 Windows persistence implementation only uses `MoveFileExW`;
it can reject replacing this open target with `Access is denied (os error 5)`.
The GitHub Windows regression reproduces that failure with the actual native
MP3 decoder. Stopping playback is not a dependable workaround for source-handle
lifetime or another program's locks.

Version 2.2.1 closes the staging handle and publishes with Rust 1.100.0's
`std::fs::rename`. On Windows that implementation tries `MoveFileExW`, then
`SetFileInformationByHandle`/`FileRenameInfoEx` with replacement and POSIX flags
after access denied. Windows 10 version 1607 and later can support this behavior,
depending on the filesystem. The temporary attribute is cleared before
publication, and cleanup ownership is retained until the rename succeeds.

The writer retries short access/sharing/lock failures for at most 1.12 seconds
of backoff. It does not delete the original first, truncate it, relax filesystem
permissions or ignore a failed replacement. A persistent lock or access denial
still returns an error; Quick Tag retains the edits. Its error dialog and the
application log include the complete cause and file path, matching Edit Tags.

Regression coverage includes successive confirmed saves with a real native
decoder open before playback, a simulated external handle denying replacement,
successful saving after that handle closes, short-lived Windows locks, complete
tag readback and unchanged original bytes after failed publication. Tests run
on GitHub Windows, Linux and both macOS targets. An external lock test is Windows
specific; it must not be replaced with a browser mock.

If access still fails, try saving a copy in a writable folder and check which
program or folder policy denies modifying/deleting the original file. The full
Windows error is required to distinguish that case from decoding or tag parsing.

References: [Rust rename](https://doc.rust-lang.org/std/fs/fn.rename.html),
[Rust 1.100.0 Windows implementation](https://github.com/rust-lang/rust/blob/1.100.0/library/std/src/sys/fs/windows.rs),
[tempfile 3.27.0 Windows implementation](https://github.com/Stebalien/tempfile/blob/v3.27.0/src/file/imp/windows.rs),
and [Rust Windows file sharing](https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html).
