# Security and single-host operations

This document describes implemented boundaries and deployment preparation, not an external audit, production rollout or completeness guarantee.

## Implemented boundaries

Passwords use Argon2id with per-password random salts and bounded hashing concurrency. Session credentials and action tokens use independent 32-byte randomness; database authentication records store digests, not plaintext tokens. Email links necessarily contain an action token in transit to the mailbox. The bounded in-memory mail queue temporarily contains the message; SMTP must be trusted and encrypted in production.

Session cookies are HttpOnly, SameSite=Strict. Production uses a Secure `__Host-starter_session` cookie (no Domain, Path=/). Development uses an explicitly different non-Secure cookie over loopback HTTP only. Every state-changing browser request needs the exact configured Origin and `X-Starter-Client: web-v1`; the header is a browser request boundary, not a secret/API key. There is no wildcard CORS. API errors/credentials/private data are not cached by the service worker.

Session absolute expiry comes from the explicit remember-me choice; idle expiry is derived from its span. Login rotates the current session. Revoke-all and password changes invalidate earlier sessions. The store locks and rechecks authorization inside writes; note ownership is never accepted from a client-supplied user ID. Notes updates/deletes require the current revision.

Email confirmation and password reset are purpose-bound, expire, and are consumed transactionally. Repeated and concurrent token use is rejected. Reissuing a reset invalidates previous reset records. Recovery requests return the same accepted status for known and unknown addresses. Registration intentionally reports duplicate accounts; do not claim complete account-enumeration resistance.

## Limits to retain in a product review

- Throttling is bounded but process-local (10 attempts/key/5 minutes, 60 global/minute, two concurrent KDF jobs). Restart resets it; multiple instances need shared controls. Edge protection and abuse monitoring are not delivered by a successful unit test.
- Mail dispatch is an in-memory, bounded queue, not a durable outbox. A crash or SMTP failure can lose delivery; the user can request a new message. Monitor sanitized delivery-failure events and replace this with durable delivery when your product needs guaranteed retry.
- No MFA, OAuth, recovery codes, email address change, organization roles or account export/deletion UI is included. Add those for your actual requirements rather than treating the starter as a compliance package.
- No breached-password database, security certification, penetration test, restore drill, live deliverability test or uptime SLA has been completed.
- The supported database transport is a dedicated **loopback** PostgreSQL connection with `sslmode=disable`, on the same trusted host. Remote/DNS DB hosts and hostaddr/options overrides are rejected. Do not remove that guard merely to connect to a managed DB; implement and test verified TLS first.
- Native Windows process-tree behavior, Safari/iOS installation behavior and assistive-technology coverage require product-specific verification. Chromium flows do not prove all browsers work.

## Local development

`deploy/compose.dev.yml` is local-only. Its example credentials and Mailpit are not production settings. Mailpit contains working verification/recovery links and must never be publicly exposed. Keep ports 55432, 8025, 1025 and the Rust bind on loopback.

`pnpm` launch helpers load `.env`; direct binaries use inherited environment only. `.env` is ignored, and examples contain no real credentials. Migration/check/prune commands are explicit. Serving does not apply schema changes.

## Manual deployment shape

The initial supported shape is one Linux host with Caddy terminating HTTPS, a single Rust process on loopback and PostgreSQL on loopback. Serve `apps/pwa/dist` as static files. The example Caddyfile is a configuration template, not an already running service.

Build using `pnpm install --frozen-lockfile`, `pnpm verify`, and `cargo build --locked --release -p starter-server`. Provision a new dedicated `starter_<product>` database and least-privilege application credentials; use a migration role for DDL where appropriate. Give the app the resulting database access without reusing unrelated BSLT or product databases.

Configure `APP_MODE=production`, `APP_ORIGIN=https://<your-real-app-host>`, `APP_BIND=127.0.0.1:8088`, a loopback `APP_DATABASE_URL`, and explicit `SMTP_HOST`, `SMTP_PORT` (STARTTLS, normally 587), `SMTP_USERNAME`, `SMTP_PASSWORD`, `MAIL_FROM`. Store these outside the repository with restricted permissions. An operator must verify certificate trust, SMTP provider access, sender-domain authentication, DNS, firewall rules and backups.

Run explicit migrations/check using the same protected environment, then supervise `starter-server serve` as an unprivileged process. Caddy requires `APP_DOMAIN` (host only) and `PWA_ROOT` (absolute built static directory). Do not use Vite's dev/preview server as production hosting. Never expose the raw API port or Mailpit as a deployment shortcut.

Validate actual login, verification, recovery, Secure cookie behavior, permission denial, logout across sessions and PWA update/offline behavior on HTTPS before accepting public users. Do not enable CDN/API response caching. Application logs must not record cookies, request bodies, action-token fragments, database URLs or SMTP credentials.

## Maintenance

Run `starter-server prune` periodically with the protected environment. Back up PostgreSQL and test restoration into a separate database. Monitor failures, mail delivery and rate-limit activity. Review dependency updates on a copy and rerun the full suite. Add new forward migrations instead of changing applied SQL/checksums.

The included CI is read-only and has no deploy job. Copying this repository does not configure hosting, secrets, SMTP accounts or GitHub template settings. It also does not automatically synchronize future starter fixes into product copies.
