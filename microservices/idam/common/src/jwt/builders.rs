/// Builder structs for `SesameAuthzClaims` and `AccessClaims`.
use super::types::{AccessClaims, JwtError, SesameAuthzClaims};

// ===========================================================================
// SesameAuthzClaimsBuilder
// ===========================================================================

pub struct SesameAuthzClaimsBuilder {
    tenant: Option<String>,
    portal: Option<String>,
    roles: Option<Vec<String>>,
    permissions: Option<Vec<String>>,
    entitlements_ref: Option<String>,
    entitlements_hash: Option<String>,
    risk: Option<String>,
}

impl SesameAuthzClaimsBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tenant: None,
            portal: None,
            roles: None,
            permissions: None,
            entitlements_ref: None,
            entitlements_hash: None,
            risk: None,
        }
    }

    pub fn tenant(mut self, tenant: impl Into<String>) -> Self {
        self.tenant = Some(tenant.into());
        self
    }

    pub fn portal(mut self, portal: impl Into<String>) -> Self {
        self.portal = Some(portal.into());
        self
    }

    #[must_use]
    pub fn roles(mut self, roles: Vec<String>) -> Self {
        self.roles = Some(roles);
        self
    }

    #[must_use]
    pub fn permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = Some(permissions);
        self
    }

    pub fn entitlements_ref(mut self, ref_str: impl Into<String>) -> Self {
        self.entitlements_ref = Some(ref_str.into());
        self
    }

    pub fn entitlements_hash(mut self, hash: impl Into<String>) -> Self {
        self.entitlements_hash = Some(hash.into());
        self
    }

    pub fn risk(mut self, risk: impl Into<String>) -> Self {
        self.risk = Some(risk.into());
        self
    }

    pub fn build(self) -> Result<SesameAuthzClaims, JwtError> {
        Ok(SesameAuthzClaims {
            tenant: self
                .tenant
                .ok_or_else(|| JwtError::MissingRequiredField("tenant".into()))?,
            portal: self
                .portal
                .ok_or_else(|| JwtError::MissingRequiredField("portal".into()))?,
            roles: self.roles.unwrap_or_default(),
            permissions: self.permissions.unwrap_or_default(),
            entitlements_ref: self.entitlements_ref,
            entitlements_hash: self.entitlements_hash,
            risk: self.risk,
        })
    }
}

impl Default for SesameAuthzClaimsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// AccessClaimsBuilder
// ===========================================================================

pub struct AccessClaimsBuilder {
    iss: Option<String>,
    sub: Option<String>,
    aud: Option<Vec<String>>,
    client_id: Option<String>,
    scope: Option<String>,
    exp: Option<i64>,
    nbf: Option<i64>,
    iat: Option<i64>,
    jti: Option<String>,
    ver: Option<u64>,
    sid: Option<String>,
    tenant_id: Option<String>,
    user_id: Option<String>,
    user_type: Option<String>,
    org_id: Option<String>,
    sx: Option<SesameAuthzClaims>,
    act: Option<super::types::ActorClaim>,
    cnf: Option<crate::dpop::DpopConfirmation>,
}

impl AccessClaimsBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            iss: None,
            sub: None,
            aud: None,
            client_id: None,
            scope: None,
            exp: None,
            nbf: None,
            iat: None,
            jti: None,
            ver: None,
            sid: None,
            tenant_id: None,
            user_id: None,
            user_type: None,
            org_id: None,
            sx: None,
            act: None,
            cnf: None,
        }
    }

    pub fn iss(mut self, iss: impl Into<String>) -> Self {
        self.iss = Some(iss.into());
        self
    }

    pub fn sub(mut self, sub: impl Into<String>) -> Self {
        self.sub = Some(sub.into());
        self
    }

    #[must_use]
    pub fn aud(mut self, aud: Vec<String>) -> Self {
        self.aud = Some(aud);
        self
    }

    pub fn client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = Some(client_id.into());
        self
    }

    pub fn scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    #[must_use]
    pub fn exp(mut self, exp: i64) -> Self {
        self.exp = Some(exp);
        self
    }

    #[must_use]
    pub fn nbf(mut self, nbf: i64) -> Self {
        self.nbf = Some(nbf);
        self
    }

    #[must_use]
    pub fn iat(mut self, iat: i64) -> Self {
        self.iat = Some(iat);
        self
    }

    pub fn jti(mut self, jti: impl Into<String>) -> Self {
        self.jti = Some(jti.into());
        self
    }

    #[must_use]
    pub fn ver(mut self, ver: u64) -> Self {
        self.ver = Some(ver);
        self
    }

    pub fn sid(mut self, sid: impl Into<String>) -> Self {
        self.sid = Some(sid.into());
        self
    }

    pub fn tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn user_type(mut self, user_type: impl Into<String>) -> Self {
        self.user_type = Some(user_type.into());
        self
    }

    pub fn org_id(mut self, org_id: impl Into<String>) -> Self {
        self.org_id = Some(org_id.into());
        self
    }

    #[must_use]
    pub fn org_id_opt(mut self, org_id: Option<String>) -> Self {
        self.org_id = org_id;
        self
    }

    #[must_use]
    pub fn sx(mut self, sx: SesameAuthzClaims) -> Self {
        self.sx = Some(sx);
        self
    }

    #[must_use]
    pub fn act(mut self, act: super::types::ActorClaim) -> Self {
        self.act = Some(act);
        self
    }

    pub fn build(self) -> Result<AccessClaims, JwtError> {
        let iss = self
            .iss
            .ok_or_else(|| JwtError::MissingRequiredField("iss".into()))?;
        let sub = self
            .sub
            .ok_or_else(|| JwtError::MissingRequiredField("sub".into()))?;
        let aud = self
            .aud
            .ok_or_else(|| JwtError::MissingRequiredField("aud".into()))?;
        let client_id = self
            .client_id
            .ok_or_else(|| JwtError::MissingRequiredField("client_id".into()))?;
        let scope = self
            .scope
            .ok_or_else(|| JwtError::MissingRequiredField("scope".into()))?;
        let exp = self
            .exp
            .ok_or_else(|| JwtError::MissingRequiredField("exp".into()))?;
        let nbf = self
            .nbf
            .ok_or_else(|| JwtError::MissingRequiredField("nbf".into()))?;
        let iat = self
            .iat
            .ok_or_else(|| JwtError::MissingRequiredField("iat".into()))?;
        let jti = self
            .jti
            .ok_or_else(|| JwtError::MissingRequiredField("jti".into()))?;
        let ver = self
            .ver
            .ok_or_else(|| JwtError::MissingRequiredField("ver".into()))?;
        let sid = self
            .sid
            .ok_or_else(|| JwtError::MissingRequiredField("sid".into()))?;
        let tenant_id = self
            .tenant_id
            .ok_or_else(|| JwtError::MissingRequiredField("tenant_id".into()))?;
        let user_id = self
            .user_id
            .ok_or_else(|| JwtError::MissingRequiredField("user_id".into()))?;
        let user_type = self
            .user_type
            .ok_or_else(|| JwtError::MissingRequiredField("user_type".into()))?;
        let sx = self
            .sx
            .ok_or_else(|| JwtError::MissingRequiredField("sx".into()))?;

        if ver == 0 {
            return Err(JwtError::MissingRequiredField("ver must be > 0".into()));
        }

        Ok(AccessClaims {
            iss,
            sub,
            aud,
            client_id,
            scope,
            exp,
            nbf,
            iat,
            jti,
            ver,
            sid,
            tenant_id,
            user_id,
            user_type,
            org_id: self.org_id,
            sx,
            act: self.act,
            cnf: self.cnf,
        })
    }
}

impl Default for AccessClaimsBuilder {
    fn default() -> Self {
        Self::new()
    }
}
