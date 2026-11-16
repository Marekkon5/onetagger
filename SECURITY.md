# Security policy

This policy applies to the maintained [fantuz/onetagger](https://github.com/fantuz/onetagger) fork. OneTagger edits audio files and can move or rename them with the permissions of the account running it. Keep backups of your library and run it as a normal user.

## Supported versions

Security fixes land on `master` and are validated by GitHub Actions. Use the latest published fork release when available, or a successful `master` build identified by its commit SHA. The inherited `1.7.0` application version alone does not identify which fork fixes a binary contains. Older workflow artifacts and upstream releases are not maintained by this fork; security backports are not guaranteed.

## Report a vulnerability privately

Use [GitHub's private vulnerability report form](https://github.com/fantuz/onetagger/security/advisories/new) when it is available. If private reporting is unavailable, use a contact method listed on the [maintainer's profile](https://github.com/fantuz) to request a private reporting channel before sending details. Do not post exploits, session links, credentials, personal library paths or music files in public Discussions or pull requests.

Include the affected commit or release, operating system and architecture, installation method, minimal reproduction steps, and the impact you observed. For network issues, describe whether you used desktop, browser or server mode, `--expose`, and a reverse proxy. Prefer a small synthetic file over private or copyrighted audio, and redact tokens, cookies and account data from logs.

Maintainers coordinate investigation, a fix or mitigation, and disclosure through the private reporting channel. This is a volunteer project; no response-time or remediation SLA is promised. Ordinary non-sensitive bugs and feature requests belong in [Discussions](https://github.com/fantuz/onetagger/discussions).

## Sessions and network access

- The server binds to loopback by default. Each process start generates a random 256-bit session capability. Desktop and browser launch links carry it in a URL fragment; the frontend removes that fragment and exchanges it for an HttpOnly, SameSite=Strict cookie. HTTPS session exchanges also set Secure. Restarting invalidates earlier sessions.
- WebSocket commands, audio and thumbnail endpoints require the session. WebSocket upgrades require a matching Origin, and loopback mode rejects unexpected Host headers. The static frontend and Spotify OAuth callback remain public routes.
- Anyone with the launch link or session cookie can use the application's filesystem features with the server account's permissions. There is no per-user authorization or filesystem sandbox. Protect terminal output containing a launch link, and do not share the link or cookies.
- `--expose` enables remote access. Use an HTTPS reverse proxy preserving the intended Host and Origin and restrict who can reach it. Plain HTTP does not protect session cookies or music data in transit. Forwarded headers are not trusted to grant access.
- WebSocket frames and messages are limited to 16 MiB. This is not an overall process memory limit or protection against every denial-of-service scenario.

## Files, providers and plugins

- Tag writes are staged beside the original and published only after successful writing and synchronization. This needs space for a second copy and does not replace backups or guarantee recovery from all filesystem or power failures.
- Tag editing follows symbolic links. Replacing a file does not update its other hard links; ownership, ACLs, extended attributes and timestamps are not guaranteed to survive. Rename and move batches report per-file outcomes and may partially complete. See [file-operation guarantees and limits](docs/SAFETY.md).
- Imported audio, tags, artwork, provider responses and playlists are untrusted input. Parser or UI vulnerabilities and unintended file access are within scope for private reports.
- Online tagging and recognition contact external providers. Do not assume these operations are offline or that provider credentials and local configuration are encrypted at rest. Review logs and configuration before sharing them.
- Provider authentication diagnostics omit raw tokens and login responses. Beatport, Beatsource, BPM Supreme and Musixmatch credential types redact their debug output; Musixmatch request errors omit token-bearing URLs. This does not scrub logs produced by older builds or sanitize custom-plugin output.
- Native custom-platform plugins execute with the application's permissions. Install plugins only from sources you trust; they are not sandboxed.

## Builds and downloads

GitHub Actions use pinned action commits, a pinned Rust toolchain, and committed Cargo/pnpm lockfiles. The pipeline runs frontend typechecks, Rust tests, Clippy and release compilation on Linux, Windows, macOS Intel and macOS ARM64. Live-provider tests are excluded from normal CI; passing builds do not certify provider availability or establish that the application has no vulnerabilities.

Build jobs have read-only repository permissions. Only the release publishing job receives repository content write access, after validation of a reviewed version bump or an explicit manual release request. Publication reuses a successful build of the current default-branch commit, verifies all four platform manifests and checksums, and leaves existing published releases and conflicting tags untouched. The publisher executes trusted default-branch code; downloaded artifacts are treated only as data. PR builds do not publish releases. Windows packages are not Authenticode signed; macOS packages are not Developer ID signed or notarized. SHA-256 checksum files help detect corruption and do not establish a signing identity.

See [development and release validation](docs/DEVELOPMENT.md) and the [installation instructions](README.md#installing).
