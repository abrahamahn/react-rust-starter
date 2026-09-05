# React + Rust Starter

A small, clone-and-own application: **one React PWA, one Rust API, PostgreSQL**.
Selected BSLT code and behavior are reused; the original BSLT workspace is not a runtime dependency. Development stays on `main`.

## Included

- Email/password registration and login, remembered sessions, logout and revoke-all.
- Real SMTP email verification, single-use password recovery, password change.
- Account display name and system/light/dark appearance.
- Private notes: owner-scoped CRUD, revision conflict protection, bounded inputs.
- Install manifest/icons, responsive UI, update notification and static offline notice.
- Explicit checksummed migrations, bounded password hashing, safe error responses.
- Locked Rust/JS dependencies and real PostgreSQL, Mailpit and Chromium regression tests.

No teams, billing, AI, product-specific workflows, separate worker service, admin app or marketing site. Notes is a removable end-to-end example, not the next product's required domain model.

**This is a development starter, not a production deployment or security certification.** HTTPS/SMTP configuration is supported for a documented single-host deployment, but live domain, email delivery, backup/restore and operating controls must be verified for each product. See [security and operations](docs/security.md). Check the [CI result for the exact commit](https://github.com/abrahamahn/react-rust-starter/actions/workflows/ci.yml); test source alone is not passing evidence.

## Run locally

Requirements: Git, Rust via rustup (the checked-in toolchain is used), Node 22.16+ or 24, pnpm 10.26.2, Docker Compose. On Linux, lettre's native TLS requires the platform OpenSSL development packages (`pkg-config` and `libssl-dev` on Ubuntu). WSL/Linux is the tested shell path.

```bash
git clone https://github.com/abrahamahn/react-rust-starter.git
cd react-rust-starter
pnpm install --frozen-lockfile
# Do this only when .env does not already exist; never overwrite your real config.
cp .env.example .env
docker compose -f deploy/compose.dev.yml up -d --wait
pnpm db:migrate
pnpm db:check
pnpm dev
```

Open **http://127.0.0.1:5178**. Do not substitute `localhost`: the configured browser Origin is intentionally exact. Development mail is delivered to **http://127.0.0.1:8025** (Mailpit), not to external recipients. Register, open the verification link from Mailpit, confirm the address, then create a private note. No real email account or API subscription is needed.

`pnpm dev` loads the root `.env`, starts Rust and Vite, and stops both on exit on WSL/Linux. React changes hot-reload; restart it after Rust changes. Native Windows process-tree shutdown is not independently validated. The Rust binary itself does not load `.env`; the Node launch scripts do. Existing shell variables take precedence.

The development database listens on loopback port 55432, API on 8088. Do not point this at BSLT or a production database. Migrations are explicit; `serve` never applies them. `docker compose down` preserves the database volume; `down -v` deletes it and is not part of normal startup.

## Commands

| Command | Purpose |
| --- | --- |
| `pnpm dev` | Rust API + Vite |
| `pnpm db:migrate` / `pnpm db:check` | Explicit schema upgrade / readiness validation |
| `pnpm auth:prune` | Remove expired authentication records |
| `pnpm verify` | Rust format/lint/unit tests + TS check + helper tests + PWA build |
| `pnpm test:e2e` | Chromium flow, with test Rust/DB/Mailpit already running |
| `python3 tests/http_integration.py` | HTTP/SQL/SMTP regressions; explicitly disposable test DB only |
| `pnpm starter:rename "My App" my-app` | Preview display-identity change |
| `pnpm starter:rename "My App" my-app --apply` | Apply display-identity change |

The full integration invocation is documented by `.github/workflows/ci.yml`. It requires `APP_MODE=test`, a loopback test database named `starter_test_<suffix>`, `psql`, and local Mailpit. It deliberately mutates test rows for expiry and migration-drift scenarios, and refuses the ordinary development database. `pnpm verify` alone is not a real-DB/browser test run.

## Structure

```text
apps/pwa/src/
  app/                       routing, shell, PWA status
  features/access/           account entry and session state
  features/account/          preferences and password change
  features/notes/            removable example UI
  lib/                       API contract and reused form helpers
  styles/                    replaceable theme and layout
apps/server/src/
  app/                       configuration and composition
  platform/access/           authentication/session lifecycle
  platform/db/               pool and explicit migration runner
  platform/http/             shared error boundary
  platform/mail/             bounded SMTP dispatcher
  modules/notes/             removable example HTTP/domain/SQL
apps/server/migrations/       one ordered history per application
tests/                       helper, HTTP/DB/SMTP, browser tests
deploy/                      local services and manual HTTPS examples
scripts/                     launch and dry-run rename helpers
```

Business code goes in `modules/<feature>` and `features/<feature>`. Keep its rules, HTTP boundary, SQL and tests close together. The platform does not acquire Kanu planning/purchasing behavior. A second crate, background job system or shared package is added only when a real feature needs it.

## Make another product

Create a new repository from a reviewed commit or use GitHub's template feature after the owner enables that repository setting. Do not assume this repository has automatically been marked as a template. Preserve [upstream provenance](docs/upstream.md); copies do not automatically receive future fixes.

Run the rename helper, replace icons and design tokens, and configure a **new** database, origin, mail sender and deployment target. Rename changes display identity only; it deliberately does not rewrite credentials, Cargo package names, migration history, database names or the cookie namespace. Use separate hostnames for separately deployed products. Products intentionally sharing a hostname need a reviewed cookie/path isolation change.

To replace Notes, remove its PWA feature and route, server module and route composition, and corresponding tests. Add your feature and migration. Do not edit a migration that has already been applied; for an existing database use a new migration. A brand-new product may design a new initial schema before its first deployment, but test that bootstrap from an empty database.

BSLT-specific package aliases, private history, environment files and deployment secrets are not copied. The owner has not selected a redistribution license for this repository; public visibility does not by itself grant a license. Dependency licenses remain their own.

## Contract and scope

[HTTP contract](contracts/README.md) · [BSLT reuse map](docs/upstream.md) · [Security and deployment](docs/security.md).

The CI workflow has read-only repository permissions and does not deploy, send real external email, create commits, or update branches. Initial lockfile/format preparation has been removed from the current workflow.
