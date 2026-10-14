CREATE TABLE mcps (
  id uuid PRIMARY KEY,
  definition jsonb NOT NULL
);
CREATE TABLE assignments (
  id uuid PRIMARY KEY,
  mcp_id uuid NOT NULL REFERENCES mcps(id) ON DELETE CASCADE,
  target_kind text NOT NULL CHECK (target_kind IN ('all','user','group')),
  target_user uuid REFERENCES users(id) ON DELETE CASCADE,
  target_group uuid REFERENCES groups(id) ON DELETE CASCADE,
  CHECK ((target_kind='all' AND target_user IS NULL AND target_group IS NULL)
      OR (target_kind='user' AND target_user IS NOT NULL AND target_group IS NULL)
      OR (target_kind='group' AND target_user IS NULL AND target_group IS NOT NULL)),
  UNIQUE NULLS NOT DISTINCT (mcp_id,target_kind,target_user,target_group)
);
CREATE INDEX assignments_user_idx ON assignments(target_user) WHERE target_user IS NOT NULL;
CREATE INDEX assignments_group_idx ON assignments(target_group) WHERE target_group IS NOT NULL;
-- Settings require ownership-constrained references before vault API delivery.
CREATE TABLE user_secrets (
  id uuid PRIMARY KEY,
  owner_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  key text NOT NULL CHECK (length(key) BETWEEN 1 AND 128),
  version bigint NOT NULL CHECK (version BETWEEN 1 AND 4294967295),
  key_id text NOT NULL,
  nonce bytea NOT NULL CHECK (octet_length(nonce)=12),
  ciphertext bytea NOT NULL CHECK (octet_length(ciphertext)>=16),
  updated_at bigint NOT NULL,
  UNIQUE (owner_id,key),
  UNIQUE (id,owner_id)
);
CREATE TABLE user_settings (
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  mcp_id uuid NOT NULL REFERENCES mcps(id) ON DELETE CASCADE,
  secret_id uuid,
  PRIMARY KEY (user_id,mcp_id),
  FOREIGN KEY (secret_id,user_id) REFERENCES user_secrets(id,owner_id) ON DELETE RESTRICT
);
CREATE INDEX settings_mcp_idx ON user_settings(mcp_id);
CREATE INDEX settings_secret_idx ON user_settings(secret_id) WHERE secret_id IS NOT NULL;
CREATE TABLE tool_inventory (
  mcp_id uuid NOT NULL REFERENCES mcps(id) ON DELETE CASCADE,
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  names text[] NOT NULL,
  observed_at bigint NOT NULL,
  PRIMARY KEY (mcp_id,user_id)
);
CREATE INDEX inventory_user_idx ON tool_inventory(user_id);
