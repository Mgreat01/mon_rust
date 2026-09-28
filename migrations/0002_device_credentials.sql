ALTER TABLE devices
    ADD COLUMN api_key_version INTEGER NOT NULL DEFAULT 1 CHECK (api_key_version > 0),
    ADD COLUMN credential_updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN credential_revoked_at TIMESTAMPTZ;

COMMENT ON COLUMN devices.api_key_hash IS
    'Hash Argon2 de la clé active. La clé en clair ne doit jamais être persistée.';
COMMENT ON COLUMN devices.api_key_version IS
    'Version monotone de la clé, incrémentée à chaque rotation.';
COMMENT ON COLUMN devices.credential_revoked_at IS
    'Date de révocation. NULL signifie que la clé courante est active.';
