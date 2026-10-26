-- OIDC client metadata `initiate_login_uri` (OpenID Connect Registration 1.0 §2): a relying party's own sign-in start.
-- Sesame's hosted login page sends a browser whose authorization request expired there, so the client begins a fresh
-- request (new state, nonce and PKCE) instead of stranding the user on "this sign-in request has expired". Stored as a
-- redirect-URI row of kind 'initiate_login', at most one per client; the page only ever uses this registered value.
--
-- Re-runnable (every release applies every file).

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_constraint
               WHERE conname = 'relying_party_client_redirect_uris_kind_check'
                 AND conrelid = 'sesame_idam.relying_party_client_redirect_uris'::regclass
                 AND pg_get_constraintdef(oid) NOT LIKE '%initiate_login%') THEN
        ALTER TABLE sesame_idam.relying_party_client_redirect_uris
            DROP CONSTRAINT relying_party_client_redirect_uris_kind_check;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint
                   WHERE conname = 'relying_party_client_redirect_uris_kind_check'
                     AND conrelid = 'sesame_idam.relying_party_client_redirect_uris'::regclass) THEN
        ALTER TABLE sesame_idam.relying_party_client_redirect_uris
            ADD CONSTRAINT relying_party_client_redirect_uris_kind_check
            CHECK (kind IN ('login', 'post_logout', 'initiate_login'));
    END IF;
END
$$;

CREATE UNIQUE INDEX IF NOT EXISTS uq_rp_redirect_one_initiate_login
    ON sesame_idam.relying_party_client_redirect_uris (relying_party_client_id)
    WHERE kind = 'initiate_login';
