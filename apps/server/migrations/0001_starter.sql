-- Fresh standalone schema. Not an upgrade of a BSLT production database.
CREATE SCHEMA starter;
CREATE TABLE starter.schema_migrations (
  version bigint PRIMARY KEY, name text NOT NULL, checksum text NOT NULL,
  applied_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE starter.users (
  id text PRIMARY KEY, email text NOT NULL UNIQUE,
  password_hash text NOT NULL, auth_epoch bigint NOT NULL DEFAULT 0,
  disabled boolean NOT NULL DEFAULT false, email_verified boolean NOT NULL DEFAULT false,
  display_name text NOT NULL DEFAULT '', theme text NOT NULL DEFAULT 'system'
    CHECK (theme IN ('system', 'light', 'dark')),
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE starter.sessions (
  token_hash text PRIMARY KEY, user_id text NOT NULL REFERENCES starter.users(id) ON DELETE CASCADE,
  auth_epoch bigint NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
  last_seen_at timestamptz NOT NULL DEFAULT now(), expires_at timestamptz NOT NULL,
  idle_seconds bigint NOT NULL CHECK (idle_seconds > 0)
);
CREATE INDEX sessions_user ON starter.sessions(user_id);
CREATE INDEX sessions_expiry ON starter.sessions(expires_at);
CREATE TABLE starter.auth_tokens (
  token_hash text PRIMARY KEY, user_id text NOT NULL REFERENCES starter.users(id) ON DELETE CASCADE,
  purpose text NOT NULL CHECK (purpose IN ('verify', 'reset')),
  expires_at timestamptz NOT NULL, used_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX auth_tokens_user ON starter.auth_tokens(user_id, purpose);
CREATE INDEX auth_tokens_expiry ON starter.auth_tokens(expires_at);
CREATE TABLE starter.notes (
  id text PRIMARY KEY, user_id text NOT NULL REFERENCES starter.users(id) ON DELETE CASCADE,
  title text NOT NULL CHECK (char_length(title) BETWEEN 1 AND 120),
  body text NOT NULL CHECK (char_length(body) <= 10000),
  revision integer NOT NULL DEFAULT 1 CHECK (revision > 0),
  created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX notes_owner ON starter.notes(user_id, created_at DESC, id DESC);
