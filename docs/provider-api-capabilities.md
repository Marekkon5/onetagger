# Provider request capabilities

Reviewed against OneTagger 2.2.0 and provider documentation on 2026-11-24.
This inventory covers all twelve built-in AutoTag providers. Custom plugins
have independent contracts; the current plugin interface accepts one track per
match/enrichment call.

A **bulk lookup** accepts several independent, already known identifiers in one
HTTP request. A **release lookup** returns several related tracks belonging to
one identified release. Neither means a list of unrelated title/artist searches
can be submitted together. A search returning fifty candidates still answers
only one query. Endpoint pagination must be completed before treating a release
as a complete tracklist.

## Inventory

The worker column records the adapter limit in 2.2.0, before the shared global
thread limit is applied. `Global` means no additional adapter cap. A worker cap
is not an HTTP quota, and one song lookup can make several HTTP requests.

| Provider | Independent song requests in one call | Related data that can be shared | Workers | Current adapter and practical action |
| --- | --- | --- | ---: | --- |
| Beatport | No verified public cross-song search/bulk-ID contract available for this adapter. | Release detail and release-tracklist endpoints already used by the adapter. | Global | Searches the website, then fetches track details. Reuse release metadata across workers; keep individual searches and exact selected release IDs. Album tagging already uses a release tracklist. |
| Beatsource | No verified public bulk-song contract for the search API used here. | Search candidates already include substantial track/release metadata. | Global | One catalog query per song; enrichment is a no-op. More search results are not a batch of separate song requests. |
| Discogs | No verified API for batching unrelated database searches. | A release/master response contains its tracklist. | 1 | Already has a per-source release cache. Keep master and release namespaces separate, bound cache lifetime/size, and pace from request starts rather than response completion. Do not replace a new song's search with a fuzzy search of arbitrary cached releases. |
| Traxsource | No verified public metadata batch API for this adapter. | Tracks on the same release share a release HTML page. | 1 | Reads search, track, and sometimes release pages. Reusing a release page can reduce repeated requests; expanding concurrency needs separate evidence of service limits. |
| MusicBrainz | No general batch of independent title/artist queries verified. | A release lookup can include recordings, artists, labels and genres; browse endpoints are paginated. | 4 | Each selected release can be fetched once for several songs. Share one proactive request budget across workers; the public service allows one call per second per client/IP. Four workers must not imply four calls per second. |
| Juno Download | No verified public batch metadata contract for this adapter. | The search page already contains release tracklists. | 4 | Parses one website query per song, including related tracks. An OR search is not an equivalent replacement: truncation and changed candidate ordering can change matches. |
| Bandcamp | No verified public cross-song catalog batch contract. | Album pages expose related tracks; current adapter retrieves a selected track page. | 4 | Uses public search and page metadata. The documented OAuth API serves label/merchandise account operations, not general catalog bulk matching. |
| Spotify | **Yes for eligible applications with known Spotify IDs:** `GET /tracks?ids=...`, at most 50 IDs. Deprecated and unavailable to newer restricted Development Mode apps. | Album/artist metadata can be reused; other several-item endpoints also depend on app eligibility. | 1 | Search already returns full track objects, so adding another bulk-track fetch would duplicate work. Album/artist reuse is more useful. Do not enable deprecated batching by default or assume every user's app can access it. |
| iTunes | **Documented ID-based multi-entity lookup**, including comma-separated AMG IDs; not a list of arbitrary song-name searches. | Album/UPC lookup with `entity=song` returns album tracks. | 1 | Current adapter only searches, and needs no detail enrichment. Known release IDs could support a separate album lookup; the Apple Music API is a different authenticated service. |
| Deezer | No verified bulk contract for the **public REST API used by OneTagger**. | `/album/{id}` supplies shared release metadata. | 2 | Reuse album metadata with language/configuration isolation. Third-party paid wrappers and private gateway/GraphQL APIs do not establish support in this adapter. |
| BPM Supreme | No verified public bulk-enrichment contract for its authenticated catalog endpoint. | Existing album search returns several track versions. | 1 | One query per song and no detail enrichment. Several versions in one album response do not represent several independent searches. |
| Musixmatch | No verified public cross-song batch contract in the official SDK's API schema. | One song's lyric/subtitle operations are already combined by the adapter's macro request. | 1 | `macro.subtitles.get` combines several operations for **one** song. The public SDK documents per-track matcher/lyric APIs and album-track listing; its API-key contract differs from this desktop adapter. |

“No verified” records an evidence limitation, not a claim that private partner
capabilities cannot exist. Private access would need provider documentation,
credentials and end-to-end validation before implementation. No endpoint/filter
is inferred solely from an HTTP 200 response: an ignored filter may return an
unrelated default page.

## Additional identification services

OneTagger's optional audio recognition uses Shazam through `songrec`, separately
from the twelve metadata adapters. No verified supported cross-song recognition
batch contract was found for that integration. It must continue to identify each
audio signature separately.

AcoustID is not a built-in adapter in this version. Its documentation does allow
batch fingerprint **submission** and batch MBID-to-AcoustID lookup. Those are not
batch identification of several unknown fingerprints and would not speed up the
current Shazam path. The public fingerprint lookup limit is three requests per
second.

Apple Music, also not a built-in adapter, documents multiple-song lookup by
known catalog IDs/ISRCs. Integrating it would require a separate authenticated
provider, matching rules and tests. It is not a switch on the existing iTunes
Search adapter.

## Accuracy and rate limits

Keep the user's provider order, candidate ordering, matching thresholds, edition
selection and per-file write ordering. A cached response is input to the existing
matcher, never permission to accept a different release. Cache only validated
successes, use exact provider/endpoint/identifier keys and isolate credentials or
response-affecting language/market options. Share concurrent identical lookups
and bound cache size/lifetime; never turn network errors or throttling into
cached “no match” results.

Packing several IDs into one HTTP request does not necessarily consume just one
quota unit. Use each provider's documented accounting and response headers.
Independent requests can overlap only within both the global worker limit and
the provider's shared budget; honor `Retry-After` rather than increasing threads
to bypass limits.

The scheduler already pipelines different songs. Providers remain ordered for
each song so a faster lower-priority provider cannot silently replace the user's
preferred result. Album mode retains its existing selection boundaries and
identity checks. Reordering providers may improve speed for a particular library,
but changes which provider wins and is a user choice.

## Primary references

- [OneTagger 2.2.0 provider implementations](https://github.com/fantuz/onetagger/tree/v2.2.0/crates/onetagger-platforms/src) and [scheduler](https://github.com/fantuz/onetagger/blob/v2.2.0/crates/onetagger-autotag/src/scheduler.rs).
- [MusicBrainz API](https://musicbrainz.org/doc/MusicBrainz_API) and [rate limiting](https://musicbrainz.org/doc/MusicBrainz_API/Rate_Limiting).
- [Discogs developer documentation](https://www.discogs.com/developers/). This page returned HTTP 403 during the review; release/search behavior above is verified in the shipped adapter. Response quota headers remain authoritative.
- [Spotify Get Several Tracks](https://developer.spotify.com/documentation/web-api/reference/get-several-tracks), [2026 migration guide](https://developer.spotify.com/documentation/web-api/tutorials/february-2026-migration-guide), and [developer access update](https://developer.spotify.com/blog/2026-02-06-update-on-developer-access-and-platform-security). The March 9 update postponed endpoint-access changes for existing integrations; Extended Quota apps are unaffected by the migration guide. Eligibility must be checked per app.
- [Apple iTunes Lookup Examples](https://developer.apple.com/library/archive/documentation/AudioVideo/Conceptual/iTuneSearchAPI/LookupExamples.html).
- [Apple Music multiple catalog songs](https://developer.apple.com/documentation/applemusicapi/get-multiple-catalog-songs-by-id).
- [Bandcamp developer API](https://bandcamp.com/developer).
- [Musixmatch official SDK schema](https://github.com/musixmatch/musixmatch-sdk/blob/master/swagger/swagger.json).
- [AcoustID web service](https://acoustid.org/webservice).

For Beatport, Beatsource, Traxsource, Juno Download, Deezer's current public REST
batch capabilities and BPM Supreme, no accessible primary bulk contract was
established during this review. The adapter source is evidence of what OneTagger
currently requests, not a guarantee about every API a provider operates.
