CREATE TABLE deployment (
  singleton boolean PRIMARY KEY CHECK (singleton),
  id uuid NOT NULL UNIQUE,
  revision bigint NOT NULL DEFAULT 0 CHECK (revision BETWEEN 0 AND 4294967295)
);
CREATE TABLE users (
  id uuid PRIMARY KEY,
  username text NOT NULL UNIQUE CHECK (length(username) BETWEEN 1 AND 64),
  password_hash text NOT NULL,
  role text NOT NULL CHECK (role IN ('admin','user')),
  active boolean NOT NULL DEFAULT true
);
CREATE TABLE sessions (
  token_hash bytea PRIMARY KEY CHECK (octet_length(token_hash) = 32),
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  csrf_token text NOT NULL,
  expires_at bigint NOT NULL
);
CREATE INDEX sessions_user_idx ON sessions(user_id);
CREATE INDEX sessions_expiry_idx ON sessions(expires_at);
CREATE TABLE groups (
  id uuid PRIMARY KEY,
  name text NOT NULL UNIQUE CHECK (length(name) BETWEEN 1 AND 128)
);
CREATE TABLE memberships (
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  group_id uuid NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
  PRIMARY KEY (user_id,group_id)
);
CREATE INDEX memberships_group_idx ON memberships(group_id);
