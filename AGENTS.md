# DOX framework

- DOX is highly performant AGENTS.md hierarchy installed here
- Agent must follow DOX instructions across any edits

## Core Contract

- AGENTS.md files are binding work contracts for their subtrees
- Work products, source materials, instructions, records, assets, and durable docs must stay understandable from the nearest applicable AGENTS.md plus every parent AGENTS.md above it

## Read Before Editing

1. Read the root AGENTS.md
2. Identify every file or folder you expect to touch
3. Walk from the repository root to each target path
4. Read every AGENTS.md found along each route
5. If a parent AGENTS.md lists a child AGENTS.md whose scope contains the path, read that child and continue from there
6. Use the nearest AGENTS.md as the local contract and parent docs for repo-wide rules
7. If docs conflict, the closer doc controls local work details, but no child doc may weaken DOX

Do not rely on memory. Re-read the applicable DOX chain in the current session before editing.

## Update After Editing

Every meaningful change requires a DOX pass before the task is done.

Update the closest owning AGENTS.md when a change affects:

- purpose, scope, ownership, or responsibilities
- durable structure, contracts, workflows, or operating rules
- required inputs, outputs, permissions, constraints, side effects, or artifacts
- user preferences about behavior, communication, process, organization, or quality
- AGENTS.md creation, deletion, move, rename, or index contents

Update parent docs when parent-level structure, ownership, workflow, or child index changes. Update child docs when parent changes alter local rules. Remove stale or contradictory text immediately. Small edits that do not change behavior or contracts may leave docs unchanged, but the DOX pass still must happen.

## Hierarchy

- Root AGENTS.md is the DOX rail: project-wide instructions, global preferences, durable workflow rules, and the top-level Child DOX Index
- Child AGENTS.md files own domain-specific instructions and their own Child DOX Index
- Each parent explains what its direct children cover and what stays owned by the parent
- The closer a doc is to the work, the more specific and practical it must be

## Child Doc Shape

- Create a child AGENTS.md when a folder becomes a durable boundary with its own purpose, rules, responsibilities, workflow, materials, or quality standards
- Work Guidance must reflect the current standards of the project or user instructions; if there are no specific standards or instructions yet, leave it empty
- Verification must reflect an existing check; if no verification framework exists yet, leave it empty and update it when one exists

Default section order:
- Purpose
- Ownership
- Local Contracts
- Work Guidance
- Verification
- Child DOX Index

## Style

- Keep docs concise, current, and operational
- Document stable contracts, not diary entries
- Put broad rules in parent docs and concrete details in child docs
- Prefer direct bullets with explicit names
- Do not duplicate rules across many files unless each scope needs a local version
- Delete stale notes instead of explaining history
- Trim obvious statements, repeated rules, misplaced detail, and warnings for risks that no longer exist

## Closeout

1. Re-check changed paths against the DOX chain
2. Update nearest owning docs and any affected parents or children
3. Refresh every affected Child DOX Index
4. Remove stale or contradictory text
5. Run existing verification when relevant
6. Report any docs intentionally left unchanged and why

## Project Contract

- **Product:** a shared, curator-led archive of long-form DJ and live mixes. One canonical mix links to independently browsable artist, crew, label and event records; provider data is stored as attributable claims; disagreements and uncertain identities stay in Review.
- **Workspace:** pnpm monorepo with `packages/domain` (shared contracts), `client` (Vue 3/Quasar), `server` (Node API + SQLite catalog), `worker` (stateless import data plane), `deploy/vps` (deployment tooling) and `tests/e2e` (Playwright).
- **Runtime:** Node 22.23.3 and pnpm 12.8.1. The catalog is a single-worker SQLite deployment; multi-worker operation is unsupported until the discovery queue is durable.
- **Source of truth:** the server catalog. Browser state is a cache/migration source; a write must never appear committed before the API returns the committed record.
- **Secrets:** provider credentials and curator material stay in private `deploy/vps/.env` or the worker environment. Never put them in the browser bundle, claim payloads, logs or fixtures.
- **Evidence rules:** uploader/channel is not a performer; upload date is not a recording date; an avatar or generic logo is not a cover; a matching name alone does not establish an identity or role.
- **Import rights:** `IMPORT_GLOBAL_RIGHTS_CONFIRMATION` defaults to true, so the operator attests once that they hold the rights to download/process provider content and the server injects that basis for every import. Per-mixset rights confirmation is obsolete. Providers that restrict an item still fail honestly; no protected stream is bypassed.
- **Verification baseline:** `pnpm typecheck`, `pnpm test`, `pnpm build`; `pnpm test:e2e` when the change touches route-level UI behavior.

## Workflow

- Keep the existing structure: one responsibility per server module, shared Zod contracts for every API boundary, and ordered SQL migrations.
- Prefer a failing test that reproduces a defect before changing identity, merge, review, import or persistence behavior.
- Do not weaken a shared schema to accommodate a single caller. Fix the caller or add an explicit, documented contract instead.
- Treat `docs/ARCHITECTURE.md`, `docs/PROVIDERS.md` and `docs/PRODUCTION_VERIFICATION.md` as current/historical records respectively; specs and plans under `docs/superpowers/` are historical design artifacts.
- Record audit findings with file references and reproducible evidence before making broad behavioral changes.

## User Preferences

- When asked for an audit, investigate first, report findings with concrete file/command evidence, and separate defects from recommended fixes.
- Do not rewrite dated production evidence; add a new dated record or a separate audit document instead.

## Child DOX Index

- **`packages/domain/`** — Shared Zod schemas, normalization, field definitions, evidence scoring and import policy. Own AGENTS.md at `packages/domain/AGENTS.md`.
- **`server/`** — Node API composition root, SQLite catalog, auth, provider adapters and the durable import control plane. Own AGENTS.md at `server/AGENTS.md`; nested docs: `server/src/catalog/AGENTS.md`, `server/src/imports/AGENTS.md`, `server/src/providers/AGENTS.md`.
- **`client/`** — Vue 3/Quasar public archive and curator control surface. Own AGENTS.md at `client/AGENTS.md`; nested docs: `client/src/catalog/AGENTS.md`, `client/src/views/AGENTS.md`.
- **`worker/`** — Stateless import worker: signed control-plane calls, bounded fetch, FFmpeg and private object storage. Own AGENTS.md at `worker/AGENTS.md`.
- **`deploy/vps/`** — Docker Compose, private setup routines, backup/restore and catalog bundle tooling. Own AGENTS.md at `deploy/vps/AGENTS.md`.
- **`tests/`** — Playwright end-to-end click-throughs against the real client, API and SQLite, with a deterministic provider boundary. Own AGENTS.md at `tests/AGENTS.md`.
- **`docs/`** — Architecture, provider limits, dated production evidence, design specs/plans and the codebase audit. Own AGENTS.md at `docs/AGENTS.md`.
