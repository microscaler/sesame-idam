-- Migration: org_domain_events
-- Append-only history of organisation domain claims (story 17.3). One row per state change; never updated or deleted
-- (trigger below; rows go with their claim through ON DELETE CASCADE only).

CREATE TABLE IF NOT EXISTS sesame_idam.org_domain_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id VARCHAR(64) NOT NULL,
    org_domain_id UUID NOT NULL REFERENCES sesame_idam.org_domains(id) ON DELETE CASCADE,
    at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    event VARCHAR(32) NOT NULL,
    detail JSONB NOT NULL DEFAULT '{}'
);
CREATE INDEX IF NOT EXISTS org_domain_events_claim ON sesame_idam.org_domain_events (org_domain_id, at);

CREATE OR REPLACE FUNCTION sesame_idam.org_domain_events_append_only()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
BEGIN
    -- A cascade from deleting the claim (or its organisation) is allowed: the history goes with what it describes.
    IF TG_OP = 'DELETE' AND pg_catalog.pg_trigger_depth() > 1 THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'org_domain_events is append-only' USING ERRCODE = '42501';
END;
$$;

DROP TRIGGER IF EXISTS org_domain_events_append_only ON sesame_idam.org_domain_events;
CREATE TRIGGER org_domain_events_append_only
    BEFORE UPDATE OR DELETE ON sesame_idam.org_domain_events
    FOR EACH ROW EXECUTE FUNCTION sesame_idam.org_domain_events_append_only();
