DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'iot_app') THEN
        CREATE ROLE iot_app
            LOGIN
            PASSWORD 'iot_app_dev_password'
            NOSUPERUSER
            NOCREATEDB
            NOCREATEROLE
            NOINHERIT
            NOBYPASSRLS;
    END IF;
END
$$;

GRANT CONNECT ON DATABASE iot_db TO iot_app;
