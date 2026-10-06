# Provider behavior and limits

## Freeteknomusic

Endpoint: `https://archive.freeteknomusic.org/`

The adapter stays on the archive origin, resolves relative URLs safely, supports bounded recursion (`maxDepth <= 6`), caps total work (`maxItems <= 2500`), ignores obvious system/artwork files and derives initial identity hints from path + filename. A plain artist/directory name first checks the matching archive folder. When a minimum duration is requested, discovery probes audio duration and excludes shorter files; it does not download the full recording. An indexed mix with a missing duration is probed during enrichment. An explicit waveform action decodes the full recording, with a 250 MiB and five-minute limit.

## SoundCloud

Uses the official public API for search. Configure `SOUNDCLOUD_CLIENT_ID` and `SOUNDCLOUD_CLIENT_SECRET` to obtain and cache a client-credentials token for public-resource queries. `SOUNDCLOUD_ACCESS_TOKEN` remains a legacy alternative, but expires and cannot renew itself. The adapter requests playable tracks, applies a duration lower bound and reads only metadata returned by the API. Upload time is not treated as recording time, and uploader avatars are not treated as mix covers.

To obtain credentials, sign in to SoundCloud and follow its [app registration guide](https://developers.soundcloud.com/docs/api/register-app). SoundCloud currently requires Artist Pro for API app registration. Public-resource search uses the [client-credentials flow](https://developers.soundcloud.com/docs/api/guide); a user OAuth login inside Mixsets is unnecessary.

If you do not have Artist Pro, open an indexed mix with missing artwork and choose **Add cover from public link**. Paste the exact public track URL, inspect its title and cover preview, then confirm it is the same mix. The worker uses SoundCloud's oEmbed endpoint without API credentials; uploader avatar fallbacks are rejected. The link and cover are saved only after confirmation, and existing canonical artwork is never replaced. An already-linked public SoundCloud track can also supply artwork during normal enrichment. Automatic SoundCloud search across other mixes still requires app credentials.

No HTML scraping, access-control bypass or protected stream extraction is implemented.

## YouTube

Known public video URLs use YouTube's oEmbed metadata and thumbnail without credentials. Text discovery and provider-wide enrichment use the official YouTube Data API v3 with `YOUTUBE_API_KEY`; the search result is checked against video details and the configured minimum duration. Upload date is never treated as recording date. A channel/uploader is never promoted to a performing artist. Title parsing may provide an identity suggestion, but uncertain performer matches remain in Review. Video thumbnails are selected automatically only when title and compatible duration establish a strong same-recording match; other images stay reviewable.

Enable YouTube Data API v3 in [Google Cloud](https://developers.google.com/youtube/v3/getting-started), create an API key restricted to that API, then run the private `youtube` setup routine below. Public search does not require a user OAuth login.

## hearthis.at

Uses the public `api-v2.hearthis.at` search and track endpoints; no account is required. Only images under the track-image path are considered cover artwork. Uploader avatars and unrelated broad search results are not used as canonical covers. Public API rate limits can temporarily make search unavailable. Known public track URLs can still be previewed and linked from the detail view.

## Internet Archive

Uses `/advancedsearch.php` for discovery and `/metadata/{identifier}` for item metadata/files. It captures creator/title/date/description/subjects/collections and identifies plausible audio/image files without downloading complete audio.

## Discogs

Used for entity enrichment rather than long-mix track matching. `DISCOGS_TOKEN` is optional but strongly recommended for rate limits. Crew/sound-system names are searched as artist-like entities; labels as labels. A unique exact search result is hydrated from the artist/label entity endpoint so profile text, image, and source URL are real. Artist images are stored on entity profiles, never as mix cover art. Conflicting IDs and profile claims remain reviewable.

Generate a personal API token under [Discogs Developer settings](https://www.discogs.com/settings/developers). A user OAuth flow is unnecessary for these public artist and label lookups.

For already-linked entities, enrichment hydrates `/artists/{id}` or `/labels/{id}` directly. Shared profiles and role-specific images belong to the entity record and are visible from every linked mix — one entity update therefore appears on every mix that references it. Matching names do not create a role or relationship. A Discogs label result alone does not establish that a label released or organized a particular mix.

## How provider data becomes canonical

Provider output is never copied straight into a record. It arrives as a typed field claim naming the target record, the field, the normalized value, the provider, the source URL, the resource identity, when it was observed and why the match was made. A claim is then either selected, treated as agreeing with an existing value, or held in Review.

- An authoritative field on a confirmed provider identity, or a curator decision, may fill a missing field.
- An equivalent value corroborates the existing one instead of creating a second canonical value.
- A material disagreement keeps the selected value and opens a field-level Review item holding both values and both sources.
- Repeated evidence is identified by a stable fingerprint, so re-running enrichment does not duplicate pending items or resurrect previously rejected claims.
- A provider failure leaves fields missing and is reported as a failure. It is never presented as "nothing new found".

The claim behind each selected value is retrievable from the record's detail view, so a reader can check which source established a field and why a competing claim was not used.

## Duplicate merges

When a curator merges two confirmed duplicates, the survivor receives the other's source identities, claims, review decisions, media and relationships. Fields both records state differently become Review items rather than being resolved by the merge order, and the retired record's URL keeps resolving to the survivor. The same normalized name is only ever a *suggestion* for a merge — artists with the same name stay distinct unless a provider identity or the curator confirms otherwise.

## Private VPS-L setup

The public Providers screen offers these instructions without accepting secrets. From a private terminal on VPS-L, run one of the commands below in the deployment checkout:

```bash
cd /opt/syco23-mixsets
python3 deploy/vps/setup_providers.py soundcloud
python3 deploy/vps/setup_providers.py discogs
python3 deploy/vps/setup_providers.py youtube
```

Run each credentialed provider separately. The routine prompts without echoing credentials, validates them with the provider before saving, writes `deploy/vps/.env` with owner-only permissions, recreates the API container, and checks its health. A read-only check is available with `python3 deploy/vps/setup_providers.py youtube --check` (or `soundcloud --check` or `discogs --check`). Do not commit `.env` or paste credentials into the browser.
