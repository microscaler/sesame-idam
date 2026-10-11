-- Migration: org_domains
-- Organisation domain claims (story 17.3, DESIGN-org-domain-verification.md §8). Hand-written around the Lifeguard
-- model (models/org_domain.rs): tenant_id is added nullable, backfilled from the organisation and only then made
-- NOT NULL; the old writable "verified" flag (never proven) is dropped after every existing row is set to PENDING.
-- Re-runnable: every release applies it again.

ALTER TABLE sesame_idam.org_domains ADD COLUMN IF NOT EXISTS auto_join BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE sesame_idam.org_domains ADD COLUMN IF NOT EXISTS created_by UUID;
ALTER TABLE sesame_idam.org_domains ADD COLUMN IF NOT EXISTS dnssec BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE sesame_idam.org_domains ADD COLUMN IF NOT EXISTS epoch INTEGER NOT NULL DEFAULT 1;
ALTER TABLE sesame_idam.org_domains ADD COLUMN IF NOT EXISTS grace_until TIMESTAMP WITH TIME ZONE;
ALTER TABLE sesame_idam.org_domains ADD COLUMN IF NOT EXISTS last_checked_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE sesame_idam.org_domains ADD COLUMN IF NOT EXISTS last_verified_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE sesame_idam.org_domains ADD COLUMN IF NOT EXISTS pending_until TIMESTAMP WITH TIME ZONE;
ALTER TABLE sesame_idam.org_domains ADD COLUMN IF NOT EXISTS scope VARCHAR(16) NOT NULL DEFAULT 'exact';
ALTER TABLE sesame_idam.org_domains ADD COLUMN IF NOT EXISTS state VARCHAR(16) NOT NULL DEFAULT 'PENDING';
ALTER TABLE sesame_idam.org_domains ADD COLUMN IF NOT EXISTS tenant_id VARCHAR(64);
ALTER TABLE sesame_idam.org_domains ADD COLUMN IF NOT EXISTS verified_at TIMESTAMP WITH TIME ZONE;

DO $$
BEGIN
    -- The old flag claimed verification without proof: every row it marked starts again at PENDING.
    IF EXISTS (SELECT 1 FROM information_schema.columns
               WHERE table_schema = 'sesame_idam' AND table_name = 'org_domains' AND column_name = 'verified') THEN
        UPDATE sesame_idam.org_domains SET state = 'PENDING', verified_at = NULL, last_verified_at = NULL;
        ALTER TABLE sesame_idam.org_domains DROP COLUMN verified;
    END IF;
END
$$;

UPDATE sesame_idam.org_domains d
   SET tenant_id = o.tenant_id
  FROM sesame_idam.organizations o
 WHERE o.id = d.org_id AND d.tenant_id IS NULL;
-- A claim whose organisation is gone has no tenant to belong to (the FK cascades; this is belt and braces).
DELETE FROM sesame_idam.org_domains WHERE tenant_id IS NULL;
ALTER TABLE sesame_idam.org_domains ALTER COLUMN tenant_id SET NOT NULL;
UPDATE sesame_idam.org_domains SET pending_until = created_at + interval '7 days'
 WHERE state = 'PENDING' AND pending_until IS NULL;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'org_domains_state_check') THEN
        ALTER TABLE sesame_idam.org_domains ADD CONSTRAINT org_domains_state_check
            CHECK (state IN ('PENDING', 'VERIFIED', 'GRACE', 'REVOKED', 'EXPIRED', 'CONFLICT'));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'org_domains_scope_check') THEN
        ALTER TABLE sesame_idam.org_domains ADD CONSTRAINT org_domains_scope_check
            CHECK (scope IN ('exact', 'subdomains'));
    END IF;
END
$$;

-- One holder per tenant: while a claim is VERIFIED or GRACE no other organisation of the tenant can hold the domain.
-- The final arbiter of a verification race (the loser becomes CONFLICT). Tenants are isolated: two may each hold it.
CREATE UNIQUE INDEX IF NOT EXISTS org_domains_one_holder
    ON sesame_idam.org_domains (tenant_id, domain) WHERE state IN ('VERIFIED', 'GRACE');
CREATE INDEX IF NOT EXISTS org_domains_tenant_domain ON sesame_idam.org_domains (tenant_id, domain);
CREATE INDEX IF NOT EXISTS org_domains_org ON sesame_idam.org_domains (org_id);
