# Starter HTTP contract

The executable contract is Rust request/response types plus the regression fixtures in `tests/http_integration.py` and `tests/e2e/starter.spec.ts`. This small application does not retain BSLT's full contract generator or claim to implement Kanu's API.

All JSON mutations reject unknown request fields. Browser requests include cookies (`credentials: same-origin`), the exact `APP_ORIGIN`, and `X-Starter-Client: web-v1`. No automatic mutation retries. Successful no-content responses are 204; errors have `{ "error": { "code": "..." } }`. API responses are `Cache-Control: no-store`.

| Method and path | Input / behavior |
| --- | --- |
| POST `/api/auth/register` | email, password, optional remember_me; 201 + unverified session and queued verification mail |
| POST `/api/auth/login` | same shape; 200 + rotated current session |
| GET `/api/auth/session` | session with user, absolute expiry and idle timeout; otherwise 401 |
| POST `/api/auth/logout` | current session revoked; absent cookie is idempotent |
| POST `/api/auth/logout-all` | all prior sessions revoked; requires current session |
| POST `/api/auth/request-verification` | email; generic 202, only an eligible account gets mail |
| POST `/api/auth/verify` | token; one-time confirmation, 204; does not create a new login |
| POST `/api/auth/forgot-password` | email; generic 202 |
| POST `/api/auth/reset-password` | token, password; one-time reset, revokes sessions, 204 |
| POST `/api/auth/change-password` | current_password, password; valid session/current password required; revokes sessions, 204 |
| PATCH `/api/account` | display_name, theme (`system`, `dark`, `light`); updated public user |
| GET `/api/notes` | verified owner only, at most 100 notes |
| POST `/api/notes` | title, body; 201; no client owner ID |
| PUT `/api/notes/{id}` | title, body, revision; 200; stale revision 409 |
| DELETE `/api/notes/{id}` | revision; 204; stale revision 409 |
| GET `/health/live` | process liveness |
| GET `/health/ready` | DB and checksummed schema readiness; never repairs state |

Passwords are preserved exactly, 15–128 Unicode characters and at most 512 UTF-8 bytes when created/changed. Email is limited to the documented ASCII subset and normalized. Sessions default to 12 hours, or 30 days when `remember_me=true`; idle timeout is min(span, seven days). Unknown remember_me defaults false. No silent unlimited session.

Email links use `/#verify=<token>` or `/#reset=<token>`; the UI removes the fragment and keeps the token in memory until explicit confirmation. Do not turn a GET or email scanner request into an account mutation. Recovery success returns to login rather than automatically logging in.

Private notes requires `email_verified`. Cross-user IDs return 404. Notes has bounded title/body/count and revision checking; 100 notes is an intentional example limit, not a claim of a complete general-purpose records API. Users, passwords and notes are stored in a new `starter` schema, with no migration of existing BSLT accounts.

See Rust types for exact field names and tests for failure cases. A new feature must add matching client validation and real HTTP/owner-denial tests. Avoid changing an endpoint in the UI alone.
