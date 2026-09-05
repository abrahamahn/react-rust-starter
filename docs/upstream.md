# Selected upstream reuse

Source: owner-authorized BSLT snapshot `6e83c445d899c3d98bd5e4a0c014279ba8e30989`. This is a selected source extraction, not a fork of the private monorepo history. Do not copy its environment files, account data or optional commercial integrations.

| Source path in BSLT | Starter destination | Treatment |
| --- | --- | --- |
| `main/apps/server-rust/src/platform/access/crypto.rs` | `apps/server/src/platform/access/crypto.rs` | Reuse Argon2id parameters, independent salts, opaque randomness/digests and password/token tests; adapt error boundary |
| `main/apps/server-rust/src/platform/access/service.rs` | corresponding starter service | Reuse bounded KDF permit lifetime and process-local throttling; extend account lifecycle |
| `main/apps/server-rust/src/platform/access/store.rs` | corresponding starter store | Adapt explicit SQL, login recheck/user locking and revocation; independent new schema |
| `main/apps/server-rust/src/platform/db/mod.rs` | `apps/server/src/platform/db/mod.rs` | Adapt pooled access, ordered migration/checksum verification and advisory migration lock |
| `main/apps/server-rust/src/app/config.rs` | `apps/server/src/app/config.rs` | Adapt injectable validation, scoped database and loopback-only NoTLS boundary |
| `main/shared/src/modules/core/auth/auth.session.logic.ts` | `apps/server/src/platform/access/session_policy.rs` | Port default 12-hour / remembered 30-day span and idle=min(span, 7 days); never copy old preview's arbitrary 30-minute timeout |
| `main/server/core/src/auth/password/service.ts` | starter access store/service | Reference purpose-bound hashed action tokens, replacement invalidation and session revocation. Consumption is rechecked under a transaction; simultaneous reset attempts cannot both succeed |
| `main/client/react/src/utils/createFormHandler.ts` | `apps/pwa/src/lib/createFormHandler.ts` | Reuse callback/error/finally structure; move onStart into protected try/finally |
| `main/client/react/src/hooks/useFormState.ts` | `apps/pwa/src/lib/useFormState.ts` | Reuse form-state pattern with a synchronous in-flight guard |
| `main/apps/server-rust/tests/access.rs` | Rust unit tests + `tests/http_integration.py` + browser suite | Carry selected normalization, rotation, logout, expiry, disabled-account and hashing scenarios; extend verification, reset races, owner CRUD and revision conflicts |

The frontend does NOT simply copy every BSLT screen. Existing full auth pages depend on its client engine, generated contracts, routes and optional providers. This starter reuses independent helpers and interaction patterns, and supplies a small client for its explicit Rust contract instead of retaining those dependency chains. The new account screens, notes example, static-only service worker, mail dispatcher and standalone tooling are additions/adaptations, not claimed as byte-identical BSLT code.

## Deliberate differences

One opaque server session replaces the old TS JWT/refresh-client arrangement; this does not migrate or authenticate BSLT users. Notes requires verified email. Verification and recovery use independent single-use records and fragment links, not a link that silently signs a user in. Settings are only display name and appearance. No roles, team model, billing, notifications platform, storage/media suite or optional starter-profile runtime is brought in.

The database bootstrap is new because this is a new product with no existing users. It does not rewrite BSLT's applied migrations. Once this starter's migration is applied, its checksum remains authoritative.

## Reuse workflow

Before implementing another common feature, locate its BSLT owner and tests, then choose direct reuse, behavior port, adaptation or explicit exclusion. Record only material differences here. Do not reread the whole monorepo for every small change, and do not assume old code is correct solely because it exists.

Future bug fixes are reviewed patches, not an automatic dependency on another repository's main. Maintain one starter implementation on this repository's main; product-specific changes stay in product copies. No quantitative token or development-time savings have been measured.
