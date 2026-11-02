-- Pre-release identity contract change: never delete or invent profiles for old accounts.
DO $$ BEGIN
  IF EXISTS (SELECT 1 FROM users) THEN
    RAISE EXCEPTION 'Email identity requires a fresh pre-release database; existing accounts are not migrated. Preserve old data and create a fresh deployment.';
  END IF;
END $$;
ALTER TABLE users RENAME COLUMN username TO email;
ALTER TABLE users DROP CONSTRAINT users_username_check;
ALTER INDEX users_username_key RENAME TO users_email_key;
ALTER TABLE users ADD COLUMN name text NOT NULL;
ALTER TABLE users ADD CONSTRAINT users_email_check
  CHECK (octet_length(email) BETWEEN 3 AND 254 AND email = lower(btrim(email)));
ALTER TABLE users ADD CONSTRAINT users_name_check
  CHECK (octet_length(name) BETWEEN 1 AND 200 AND name = btrim(name));
