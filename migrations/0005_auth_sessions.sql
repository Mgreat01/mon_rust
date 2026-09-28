ALTER TABLE users
    ADD COLUMN failed_login_attempts INTEGER NOT NULL DEFAULT 0 CHECK (failed_login_attempts >= 0),
    ADD COLUMN locked_until TIMESTAMPTZ,
    ADD COLUMN password_changed_at TIMESTAMPTZ NOT NULL DEFAULT now();

CREATE TABLE auth_refresh_tokens (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    family_id UUID NOT NULL,
    token_hash BYTEA NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ
);
CREATE INDEX auth_refresh_tokens_user ON auth_refresh_tokens (tenant_id,user_id,created_at DESC);

CREATE TABLE password_reset_tokens (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash BYTEA NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX password_reset_tokens_user ON password_reset_tokens (tenant_id,user_id,created_at DESC);

ALTER TABLE auth_refresh_tokens ENABLE ROW LEVEL SECURITY;
ALTER TABLE auth_refresh_tokens FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON auth_refresh_tokens
    USING (tenant_id = app_private.current_tenant_id())
    WITH CHECK (tenant_id = app_private.current_tenant_id());

ALTER TABLE password_reset_tokens ENABLE ROW LEVEL SECURITY;
ALTER TABLE password_reset_tokens FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON password_reset_tokens
    USING (tenant_id = app_private.current_tenant_id())
    WITH CHECK (tenant_id = app_private.current_tenant_id());

DROP FUNCTION app_private.user_for_login(TEXT);
CREATE FUNCTION app_private.user_for_login(requested_email TEXT)
RETURNS TABLE (
    id UUID, tenant_id UUID, email VARCHAR(255), password_hash TEXT,
    role VARCHAR(20), is_active BOOLEAN, failed_login_attempts INTEGER,
    locked_until TIMESTAMPTZ
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
    SELECT u.id,u.tenant_id,u.email,u.password_hash,u.role,u.is_active,
           u.failed_login_attempts,u.locked_until
    FROM public.users AS u
    WHERE lower(u.email)=lower(requested_email)
    LIMIT 1
$$;

CREATE FUNCTION app_private.session_for_refresh(requested_hash BYTEA)
RETURNS TABLE (
    token_id UUID, tenant_id UUID, user_id UUID, family_id UUID,
    expires_at TIMESTAMPTZ, revoked_at TIMESTAMPTZ,
    email VARCHAR(255), role VARCHAR(20), is_active BOOLEAN
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
    SELECT t.id,t.tenant_id,t.user_id,t.family_id,t.expires_at,t.revoked_at,
           u.email,u.role,u.is_active
    FROM public.auth_refresh_tokens AS t
    JOIN public.users AS u ON u.id=t.user_id AND u.tenant_id=t.tenant_id
    WHERE t.token_hash=requested_hash
    LIMIT 1
$$;

CREATE FUNCTION app_private.reset_token_for_use(requested_hash BYTEA)
RETURNS TABLE (
    token_id UUID, tenant_id UUID, user_id UUID,
    expires_at TIMESTAMPTZ, used_at TIMESTAMPTZ
)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
    SELECT t.id,t.tenant_id,t.user_id,t.expires_at,t.used_at
    FROM public.password_reset_tokens AS t
    WHERE t.token_hash=requested_hash
    LIMIT 1
$$;

REVOKE ALL ON FUNCTION app_private.user_for_login(TEXT) FROM PUBLIC;
REVOKE ALL ON FUNCTION app_private.session_for_refresh(BYTEA) FROM PUBLIC;
REVOKE ALL ON FUNCTION app_private.reset_token_for_use(BYTEA) FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname='iot_app') THEN
        GRANT EXECUTE ON FUNCTION app_private.user_for_login(TEXT) TO iot_app;
        GRANT EXECUTE ON FUNCTION app_private.session_for_refresh(BYTEA) TO iot_app;
        GRANT EXECUTE ON FUNCTION app_private.reset_token_for_use(BYTEA) TO iot_app;
        GRANT SELECT,INSERT,UPDATE,DELETE ON auth_refresh_tokens,password_reset_tokens TO iot_app;
    END IF;
END
$$;
