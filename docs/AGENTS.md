# docs — durable project records

## Purpose

Hold the current architecture, provider behavior/limits, dated production evidence, historical design specs/plans and audit records.

## Ownership

- `ARCHITECTURE.md` — current runtime, persistence, identity, evidence, merge, access and import-plane design.
- `PROVIDERS.md` — per-provider behavior, credentials and limits.
- `PRODUCTION_VERIFICATION.md` — dated rollout and runtime evidence; historical records are not rewritten.
- `MIGRATION_FROM_ONETAGGER.md` — what was reused and retired from the upstream product.
- `superpowers/specs/`, `superpowers/plans/` — approved historical design and implementation plan, including documented drift.
- `AUDIT.md` — dated codebase audit, findings, evidence, remediation priorities and the additive remediation record.

## Local Contracts

- `ARCHITECTURE.md` describes current behavior. If code changes behavior, update it in the same change.
- `PRODUCTION_VERIFICATION.md` is append-only dated evidence. Add a new section; do not retroactively change older measurements.
- Specs/plans are historical. Mark implementation drift explicitly instead of presenting the plan as current code.
- Audit documents separate confirmed defects, inspected risks and recommended fixes. Cite file paths and line-level evidence, and never include credentials.
- Provider limits must match `docs/PROVIDERS.md`, `PROVIDER_CAPABILITIES` and the adapters.

## Work Guidance

- Keep docs operational: what a maintainer must do, not a diary.
- When a documented guarantee is contradicted by code, either fix the code or correct the doc immediately.
- Run the commands a doc claims were run, or mark them historical.

## Verification

No automated docs check. Validate referenced commands with `pnpm typecheck`, `pnpm test`, `pnpm build` or `pnpm test:e2e` as applicable.

## Child DOX Index

- **`superpowers/`** — historical design specs and implementation plans. No nested AGENTS.md; treat as archival source material.
