# SYCO23 normalized archive and enrichment

Date: 2026-10-03

Status: conversational design and written specification approved by the user.

## Outcome

Provide one shared, persistent archive at mixsets.syco23.org. Each recording has one canonical mix entry, linked to independently browsable artist, crew, label and event indexes. Provider data becomes validated, attributable claims. Proven matches fill missing fields; conflicting claims and uncertain identities remain in Review until a curator decides.

The dataset may contain missing fields when reliable sources do not provide them. The UI must distinguish missing, unsupported, unavailable and disputed information. It must never report that all metadata is present merely because an enrichment run found nothing new.

## Current implementation and verified constraints

- The Vue/Quasar client stores mixes, candidates and embedded entity profiles in browser localStorage. Another browser cannot resolve those mix links.
- The Node worker has discovery adapters for Freeteknomusic, Archive.org, SoundCloud, YouTube and hearthis.at, plus Discogs entity enrichment. It has no persistent catalog or separate entity indexes.
- The existing browser duplicate check includes a provider-unscoped external ID comparison. This can confuse unrelated records from different providers.
- Discogs artist and label endpoints provide entity profiles, images and explicit entity references. An artist portrait is separate from mix artwork; release country is separate from an artist/crew/label's country.
- SoundCloud public-link artwork lookup works without app credentials. Its [official search requires OAuth credentials, including a client secret](https://developers.soundcloud.com/docs/api/guide#client-credentials-token-exchange-flow). A client ID alone returned HTTP 401 during this audit.
- The supplied YouTube key passed lookup and search checks and is configured privately on VPS-L. A production enrichment request for Kan10 / Live Mackitek Koalisson III returned its matching video cover, source ID and Discogs artist profile. The supplied replacement Discogs token returned HTTP 401; the existing authenticated token was preserved.
- Both production domains and the frontend-to-API CORS origin were verified. Provider credentials remain on the worker and must never enter frontend configuration, raw claim payloads or logs.

## Architecture

Use SQLite on the existing VPS-L worker, with a persistent Docker volume for the database and generated media. Keep Vercel as the frontend host. Use Node's built-in SQLite support with a pinned, compatible Node 22 runtime; use parameterized statements, foreign keys, uniqueness constraints and short transactions. Network requests and audio decoding happen outside database transactions.

Add a shared TypeScript domain package containing Zod schemas, inferred types, normalization functions and field definitions. Both client and server use the same schemas for canonical records, provider claims, import payloads and API responses. Provider adapters validate remote data before it enters the catalog; the API validates untrusted inputs and returns field-specific errors.

Separate modules own validation/normalization, catalog persistence, identity resolution, claim merging, provider adaptation, authentication and HTTP routes. The existing discovery and waveform implementations are reused through these interfaces.

SQLite is appropriate for the current single worker. The repository interface must allow a later database replacement without changing provider or merge behavior. A managed Postgres service was considered; it adds another operated service without a current deployment requirement.

## Canonical model and indexes

Every canonical record has a stable opaque ID, creation/update timestamps, revision, review state and selected field evidence. A common entity identity can have explicitly confirmed artist, crew and/or label roles. Those roles produce separate indexes and have separate type-specific details; an organization that is both a crew and label shares one identity rather than receiving duplicate identities. Matching names alone do not establish an additional role.

Record identity has an explicit verification state: proposed, source-confirmed or curator-confirmed. Default public entity/event indexes contain confirmed identities; unresolved proposals remain visible to the curator in Review. Individual fields retain their own evidence state, so a confirmed identity does not imply that every optional field is proven. A name inferred from a filename cannot establish a confirmed artist or crew identity by itself.

| Index | Canonical data |
| --- | --- |
| Mixes | Title, performer/crew/label links, event links, description, duration in milliseconds, recording date with precision, genres/styles, cover assets, playback sources and analyzed waveform |
| Artists | Display name, aliases, profile, real name when explicitly supplied, country when supported, portrait and confirmed group/member relationships |
| Crews | Display name, aliases, profile, logo, country, website/source links and confirmed memberships |
| Labels | Display name, aliases, profile, logo, country, source/website/contact information and confirmed parent/sub-label relationships |
| Events | Name, start/end date with precision, venue, locality/country, flyer assets, source links and connected mixes/entities |

Use relational tables for records, entities, entity roles, type-specific details, aliases, mixes, events, typed relationships, provider sources, media assets, field claims, selected evidence, review decisions, enrichment runs, curator sessions and legacy ID mappings. Do not store repeated complete artist profiles inside each mix.

Arrays such as genres, aliases and source links have normalized values or relation rows. Arbitrary provider JSON is retained only as a bounded, sanitized evidence snapshot; it is not the canonical schema. Optional fields use absence rather than placeholder strings such as "Unknown artist". Completeness is derived from applicable canonical fields and shared with the detail UI.

Recording date, upload date and event date are distinct fields. Date precision preserves year-only or month-only source data rather than manufacturing a day. Countries use validated ISO codes when a source establishes them. Durations must be finite, positive and within supported recording bounds. URLs are validated and canonicalized with provider-specific rules; image roles distinguish mix cover, portrait, crew/label logo and event flyer.

## Identity and duplicate rules

1. A provider identity is `(provider, resource type, external ID)`, with a database uniqueness constraint. Discogs artist and label resources are distinct namespaces. A YouTube video ID cannot collide with a hearthis.at numeric ID.
2. Canonical provider URLs also have unique identity mappings after provider-specific normalization. Preserve meaningful URL parameters and path case; remove known tracking parameters only.
3. Display text preserves spelling. Identity search keys use Unicode normalization, case and whitespace normalization. Search-only accent/punctuation folding can suggest matches but cannot silently merge them. Names such as `DJ ...` or `Live ...` must not lose meaningful words through the current search-query normalizer.
4. Repeated imports of the same source are idempotent and return the existing canonical record. Concurrent imports rely on database constraints and transactions, not browser checks.
5. Different sources are linked automatically only when the same recording is established by strong identity evidence and compatible available duration. Name/title similarity alone produces a possible duplicate in Review. Missing duration does not count as confirming evidence.
6. Artists with the same name remain distinct unless provider identity or a curator confirms equivalence. Aliases remain attributable. Events with the same name at different dates/venues remain distinct; incomplete event identity is reviewed rather than deduplicated by name alone.
7. A curator can merge confirmed duplicates transactionally. The survivor receives source identities, relationships, evidence and missing fields; disagreements remain review items. Old IDs redirect to the survivor. No selected evidence is discarded.

## Enrichment and curated merging

Provider output is adapted into typed field claims containing target record, field, normalized value, provider, source URL, resource identity, observation time, evidence category and match explanation. Confidence can prioritize review; confidence alone is not proof.

The server resolves identities, validates each claim and applies one merge policy to all five indexes:

- An authoritative field on a confirmed provider identity, or an explicitly confirmed same-recording source, may fill a missing field.
- An equivalent existing value corroborates its evidence and does not create another canonical value or review item.
- A material disagreement preserves the selected value and creates a field-level Review item with both values and their sources.
- Ambiguous entity matches, uncertain recording matches and parsed free-text facts require review before becoming canonical.
- Repeated evidence is identified by a stable claim fingerprint. Re-running enrichment does not duplicate pending items or resurrect unchanged rejected claims.
- Accepted/rejected review decisions persist with timestamps. Acceptance checks the current record revision; stale decisions must be refreshed instead of overwriting newer curation.
- Claims and resulting canonical changes are committed together. Provider failures do not roll back valid results from other providers.

Discogs enrichment operates on linked entity IDs and entity-specific gaps, independent of which mix first introduced an entity. Hydrate profiles, images, aliases and explicit entity relationships from artist/label resources. Display normalized profile text with source attribution. A linked entity update becomes visible on every mix that references it.

YouTube, SoundCloud and hearthis.at contribute recording metadata and artwork for established matching recordings. A channel/uploader is modeled as the uploader; it does not automatically become the performing artist. SoundCloud's explicit artist metadata takes precedence over an uploader-name guess. Generic provider logos, uploader avatars and unrelated images cannot fill the mix cover field. Existing good covers stay selected; a different proposed cover goes to Review.

Freeteknomusic and Archive.org contribute source-specific recording facts and permitted audio analysis. Structured, explicit event/crew/label facts can become claims. Filename or description parsing can propose those facts for Review with the supporting excerpt; a Discogs label mention alone does not prove that label released or organized a particular mix. No event date, country, crew, venue or flyer is invented to satisfy completeness.

Enrichment checks every usable provider capability for missing fields and can refresh evidence on already-linked sources. It must not skip a provider wholesale because the mix already has one source from that provider. Per-run results persist attempted providers, applied/corroborated/reviewed claims, errors and remaining missing fields. Interrupted runs are recorded as interrupted and offer a retry; they cannot leave records indefinitely marked as enriching.

## API, access and frontend

Public GET routes expose paginated/searchable mix, artist, crew, label and event indexes and detail records. Mix pages link to their related entities; entity/event pages show their sourced fields and connected mixes. Detail links work from a fresh browser. The Review screen handles field conflicts, uncertain identities and possible duplicate merges across all index types.

A single curator login controls writes, imports, provider searches, enrichment, waveform processing and review decisions. The curator password is set through a private VPS setup routine, stored as a salted password hash, and never included in the bundle. Sessions use secure HTTP-only cookies; session tokens are stored hashed server-side. Login attempts are rate limited. The API accepts credentialed requests only from configured frontend origins and verifies the origin of writes. Public readers see the archive without editing credentials.

The client uses the server catalog as its source of truth and may cache previously fetched records for browsing. Failed writes display an error and remain unsaved; offline changes must not appear committed. Add-to-index can optionally enqueue enrichment, with honest progress/results and links to Review. Each index supports browsing, search, detail, enrichment and curator corrections with evidence.

The API returns canonical records after changes. Revision checks protect simultaneous curator sessions. Client and API derive missing-field lists from the same domain definitions. The UI distinguishes "nothing added" from "no fields missing" and shows unavailable providers and fields for which no supported evidence was found.

## Migration and rollout

1. Add the durable catalog behind new API routes and deploy with a persistent data/media volume, schema migration tracking and a backup/restore routine. Back up the database consistently rather than copying a live database file alone.
2. Configure the curator privately and verify public reads, login, write protection and CORS before switching frontend storage.
3. On curator login, offer migration of the current browser's real indexed records. Validate each record, preserve curated artwork, waveform and review decisions, and deduplicate by provider identities. Exclude known demo fixtures from the shared catalog. Keep the original local data until the server confirms the batch and provide a migration summary.
4. Preserve old browser mix IDs through alias mappings, so existing detail links resolve after migration. Two browsers importing the same source converge on one canonical record. A colliding legacy ID pointing to different source identities requires review.
5. Migrate generated waveforms into persistent media assets with bounded validation. Existing audio-analysis limits remain enforced; processing never begins just to improve a completeness score.
6. Switch frontend reads/writes to the shared catalog and add the separate index views. Verify a real Mackitek import/enrichment, then verify the resulting detail pages from an independent session and after a worker restart.
7. Rollback restores the database backup and previous worker/frontend versions. Migration retains browser copies until acknowledged; rollback must not erase them.

Other users' browser-local records cannot be uploaded from this agent's session. Each browser must perform its own authorized migration; the frontend provides that routine.

## Verification and acceptance

- Shared domain schemas reject invalid provider data, bad dates/URLs/countries/durations, wrong entity types and malformed relationships. Typechecks pass for server and client.
- The same provider source imported twice or concurrently creates one mix. Equal IDs from different providers/resource types stay distinct. Unicode/alias variants suggest the right matches without conflating unrelated names.
- A known Discogs identity fills missing profile/image/entity details. Two mixes reference the same entity and both display the updated profile. Conflicting provider IDs, countries, names and images remain in Review.
- Artist/crew/label roles and membership relationships require explicit evidence or curator confirmation. Event instances stay distinct by supported identity/date/venue evidence.
- A confirmed Kan10/Koalisson YouTube match fills the missing cover automatically. An unrelated upload, uploader avatar or generic logo cannot fill it. An existing good cover, profile, date or manually curated value survives enrichment.
- Missing fields, applied fields, provider failures and review counts agree between API results and detail views. Repeating a run or rejecting a claim does not multiply Review items.
- Migration preserves real local records, covers, waveform and decisions; omits fixtures; supports retry; maps legacy links and deduplicates imports from two browsers.
- Public readers cannot mutate the catalog. Curator login/logout, origin checks, session expiry and revision conflicts work; credentials remain absent from response bodies and the frontend bundle.
- Database and generated media survive worker recreation. A consistent backup can be restored into a test instance with source identities and decisions intact.
- Build and existing meaningful provider/analysis tests pass. New tests focus on validation, identity/merge policy, persistence, migration and access. Production desktop and a controllable mobile viewport cover import → enrich → Review → shared detail and entity navigation; report any unavailable browser capability honestly.

## Explicit scope

This change establishes the shared catalog, normalized indexes, evidence/Review pipeline, migration and curator UI on the existing deployment. It does not introduce multiple user libraries, a new hosting service, automated external publishing, a separate event-data provider or speculative factual data. Providers can leave fields missing when they lack reliable information. SoundCloud-wide official search remains unavailable until its client secret or a supported access token is configured.
