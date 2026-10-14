CREATE TABLE oauth_credentials (
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  mcp_id uuid NOT NULL REFERENCES mcps(id) ON DELETE CASCADE,
  installation_id uuid NOT NULL,
  version bigint NOT NULL CHECK (version BETWEEN 1 AND 4294967295),
  key_id text NOT NULL,
  nonce bytea NOT NULL CHECK (octet_length(nonce)=12),
  ciphertext bytea NOT NULL CHECK (octet_length(ciphertext)>=16),
  refresh_holder uuid,
  refresh_until bigint,
  CHECK ((refresh_holder IS NULL) = (refresh_until IS NULL)),
  PRIMARY KEY (user_id,mcp_id,installation_id)
);
CREATE INDEX oauth_mcp_idx ON oauth_credentials(mcp_id);
CREATE INDEX oauth_key_idx ON oauth_credentials(key_id);
CREATE INDEX secrets_key_idx ON user_secrets(key_id);
