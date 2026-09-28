CREATE SCHEMA IF NOT EXISTS app_private;

CREATE OR REPLACE FUNCTION app_private.current_tenant_id()
RETURNS UUID
LANGUAGE sql
STABLE
AS $$
    SELECT NULLIF(current_setting('app.tenant_id', true), '')::UUID
$$;

REVOKE ALL ON SCHEMA app_private FROM PUBLIC;
GRANT USAGE ON SCHEMA app_private TO PUBLIC;
GRANT EXECUTE ON FUNCTION app_private.current_tenant_id() TO PUBLIC;

ALTER TABLE tenants ENABLE ROW LEVEL SECURITY;
ALTER TABLE tenants FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON tenants
    USING (id = app_private.current_tenant_id())
    WITH CHECK (id = app_private.current_tenant_id());

ALTER TABLE users ENABLE ROW LEVEL SECURITY;
ALTER TABLE users FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON users
    USING (tenant_id = app_private.current_tenant_id())
    WITH CHECK (tenant_id = app_private.current_tenant_id());

ALTER TABLE devices ENABLE ROW LEVEL SECURITY;
ALTER TABLE devices FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON devices
    USING (tenant_id = app_private.current_tenant_id())
    WITH CHECK (tenant_id = app_private.current_tenant_id());

ALTER TABLE sensor_data ENABLE ROW LEVEL SECURITY;
ALTER TABLE sensor_data FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON sensor_data
    USING (tenant_id = app_private.current_tenant_id())
    WITH CHECK (tenant_id = app_private.current_tenant_id());

ALTER TABLE alert_rules ENABLE ROW LEVEL SECURITY;
ALTER TABLE alert_rules FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON alert_rules
    USING (tenant_id = app_private.current_tenant_id())
    WITH CHECK (tenant_id = app_private.current_tenant_id());

ALTER TABLE alerts ENABLE ROW LEVEL SECURITY;
ALTER TABLE alerts FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON alerts
    USING (tenant_id = app_private.current_tenant_id())
    WITH CHECK (tenant_id = app_private.current_tenant_id());

ALTER TABLE notifications ENABLE ROW LEVEL SECURITY;
ALTER TABLE notifications FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON notifications
    USING (tenant_id = app_private.current_tenant_id())
    WITH CHECK (tenant_id = app_private.current_tenant_id());

-- Ces deux fonctions constituent les seules lectures avant établissement du
-- contexte tenant. Elles ne renvoient qu'une ligne précisément identifiée.
CREATE OR REPLACE FUNCTION app_private.user_for_login(requested_email TEXT)
RETURNS TABLE (
    id UUID,
    tenant_id UUID,
    email VARCHAR(255),
    password_hash TEXT,
    role VARCHAR(20),
    is_active BOOLEAN
)
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
    SELECT u.id, u.tenant_id, u.email, u.password_hash, u.role, u.is_active
    FROM public.users AS u
    WHERE lower(u.email) = lower(requested_email)
    LIMIT 1
$$;

CREATE OR REPLACE FUNCTION app_private.device_for_auth(requested_id UUID)
RETURNS TABLE (
    tenant_id UUID,
    api_key_hash TEXT,
    status VARCHAR(20),
    credential_revoked_at TIMESTAMPTZ
)
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
    SELECT d.tenant_id, d.api_key_hash, d.status, d.credential_revoked_at
    FROM public.devices AS d
    WHERE d.id = requested_id
    LIMIT 1
$$;

REVOKE ALL ON FUNCTION app_private.user_for_login(TEXT) FROM PUBLIC;
REVOKE ALL ON FUNCTION app_private.device_for_auth(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION app_private.user_for_login(TEXT) TO PUBLIC;
GRANT EXECUTE ON FUNCTION app_private.device_for_auth(UUID) TO PUBLIC;
