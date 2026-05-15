# Story 3.2: Implement Refresh Token Family / Reuse Detection

## Epic

[03-token-lifecycle](../tokens.md)

## Parent Epic Story

Story 3.2

## Summary

Implement token family grouping and reuse detection. Each active user session has one token family. When a token is used, the entire family is marked for reuse detection. If the same family token is used twice, all tokens are invalidated and the user must re-authenticate. This prevents the "tear" scenario where both the attacker and legitimate user have the same token.

## Why This Story Exists

The JWT document identifies refresh token rotation as insufficient without reuse detection: "Without reuse detection, a stolen refresh token can be replayed by both the legitimate user and the attacker." The "tear" scenario: attacker steals token A, legitimate user uses token A, attacker uses token A again -- both have valid tokens until the legitimate user rotates. With family-based detection, the moment token A is reused, the entire family is revoked and both parties know (legitimate user sees 401, attacker gets nothing).

## Design Context

### Token Family Structure

Each user session has a single token family. All refresh tokens issued during that session share the same `family_id`.

```
Login (Session 1)
  -> family_id: "fam_abc123"
  -> Refresh Token A (jti: "rt_001"): family_id = "fam_abc123"
  
/refresh with A
  -> family_id: "fam_abc123"
  -> Refresh Token B (jti: "rt_002"): family_id = "fam_abc123"
  -> A is added to denylist for 24h

/refresh with B
  -> family_id: "fam_abc123"
  -> Refresh Token C (jti: "rt_003"): family_id = "fam_abc123"
  -> B is added to denylist for 24h
  
If attacker uses A (already in denylist):
  -> Reuse detected for family "fam_abc123"
  -> ALL tokens in family are revoked (A, B, C)
  -> User must re-authenticate
```

### Multiple Sessions

A user can have multiple active sessions (e.g., web browser + mobile app). Each session has its own token family:

```
Session 1 (Web): family_id = "fam_abc123"
  -> Refresh Token A1 (jti: "rt_001")
  -> Refresh Token A2 (jti: "rt_002")

Session 2 (Mobile): family_id = "fam_def456"
  -> Refresh Token B1 (jti: "rt_003")
  
If attacker uses A1 (reuse on Session 1):
  -> Only Session 1 tokens are revoked (A1, A2)
  -> Session 2 tokens (B1) remain valid
  -> Web user must re-auth, mobile user continues
```

This is the correct behavior -- the tear scenario only affects the compromised session, not all sessions.

### Redis Data Structures

| Key | Type | TTL | Purpose |
|-----|------|-----|---------|
| `family:{family_id}` | Set | 24 hours | All jti values in the family |
| `family:{family_id}:revoked` | Boolean | Until cleanup | Marks family as compromised |
| `session:{family_id}` | Hash | 30 days | Session metadata |

## Implementation Notes

### Family ID Generation

```rust
pub fn generate_family_id() -> String {
    // UUID v4 for uniqueness
    format!("fam_{}", uuid::Uuid::new_v4())
}
```

### Family Lookup

On every `/refresh`:

```rust
// 1. Extract family_id from refresh token
let family_id = token.family_id;

// 2. Check if family is marked as revoked
let revoked = redis.get::<_, bool>(&format!("family:{family_id}:revoked")).unwrap_or(false);
if revoked {
    return Err(AuthError::FamilyRevoked);
}

// 3. Check if token's jti is in the family's jti set
let jti_set = redis.smembers::<_, Vec<String>>(&format!("family:{family_id}")).unwrap_or_default();
if jti_set.contains(&token.jti) {
    // Token was already used in this family -> reuse detected
    redis.set(&format!("family:{family_id}:revoked"), true).unwrap();
    // Invalidate all tokens in this family
    for other_jti in &jti_set {
        if other_jti != &token.jti {
            redis.del(&format!("refresh:{other_jti}")).ok();
        }
    }
    return Err(AuthError::FamilyRevoked);
}

// 4. Add new jti to family set
redis.sadd(&format!("family:{family_id}"), new_jti).unwrap();
```

### Logout-All vs Session-Only Logout

| Action | Affected Families | Redis Operation |
|--------|------------------|-----------------|
| Logout (single session) | One family | `DEL refresh:{jti}`, `SREM family:{family_id}:{jti}` |
| Logout-all | All families for user | For each family: revoke + remove all tokens |

## Mermaid Diagrams

### Token Family Diagram

```mermaid
graph TB
    subgraph "User: alice"
        subgraph "Session 1: Web Browser"
            S1[Session 1] --> F1[fam_abc123]
            F1 --> A1[Refresh Token A1<br/>jti: rt_001]
            F1 --> A2[Refresh Token A2<br/>jti: rt_002]
        end
        subgraph "Session 2: Mobile App"
            S2[Session 2] --> F2[fam_def456]
            F2 --> B1[Refresh Token B1<br/>jti: rt_003]
        end
    end
```

### Reuse Detection Flow

```mermaid
sequenceDiagram
    participant Client as Legitimate User
    participant Attacker as Attacker
    participant IDAM as Identity Service
    participant Redis

    Client->>IDAM: POST /refresh {token_A1}
    IDAM->>Redis: Check family:fam_abc123
    Redis-->>IDAM: Not revoked
    IDAM->>Redis: SADD family:fam_abc123 rt_002
    IDAM->>Redis: DEL refresh:rt_001
    IDAM->>Redis: SETEX denylist:rt_001 86400
    IDAM-->>Client: {token_B1, refresh_token_A3}
    
    Attacker->>IDAM: POST /refresh {token_A1}
    IDAM->>Redis: Check family:fam_abc123
    Redis-->>IDAM: Revoked!
    IDAM->>Redis: SREM family:fam_abc123 rt_002
    IDAM->>Redis: DEL refresh:rt_002
    IDAM-->>Attacker: 401 Family Revoked
    Note over Client,Attacker: Both parties know<br/>compromised session
```

### Multiple Session Isolation

```mermaid
flowchart TD
    A[Attacker uses Session 1 token] --> B{Session 1 family revoked?}
    B -->|Yes| C[Session 1: 401 re-auth required]
    B -->|No| D[Session 1: token accepted]
    
    E[Session 2 token still valid?] --> F{Session 2 family revoked?}
    F -->|No| G[Session 2: continues working]
    F -->|Yes| H[Session 2: 401 re-auth required]
    
    Note: Only the compromised session is affected<br/>Other sessions remain valid
```

### Family State Machine

```mermaid
stateDiagram-v2
    [*] --> ACTIVE: Login
    ACTIVE --> ROTATED: /refresh
    ROTATED --> ACTIVE: New token issued
    ROTATED --> REUSE_DETECTED: Token reused in family
    REUSE_DETECTED --> REVOKED: All family tokens removed
    ACTIVE --> LOGOUT: User logout
    LOGOUT --> [*]: Family removed
    REVOKED --> [*]: Family cleaned up
```

## OpenAPI Changes

- `/auth/refresh` response: Add `reason: "family_revoked"` for reuse detection
- Error response 401: Document the family_revoked reason with guidance to re-authenticate

```yaml
components:
  schemas:
    Error:
      type: object
      properties:
        reason:
          type: string
          enum: ["invalid_token", "token_expired", "token_rotated", "family_revoked", "insufficient_permissions"]
        message:
          type: string
```

## Design Doc References

- `design-doc.md` section 10.4: Token Versioning & Revocation -- Layer 2: rotating refresh tokens with reuse detection
- `design-doc.md` section 10.1: Token Security -- "Rotating token families stored in Redis with reuse detection (revoke family on replay)"
- `topics/topic-token-lifecycle.md`: (new) Document family structure

## Wiki Pages to Update/Create

- `topics/topic-token-lifecycle.md`: (new) Document family-based reuse detection
- `topics/topic-login-flow.md`: Update with multiple session handling

## Acceptance Criteria

- [ ] Each login creates a unique token family with `family_id`
- [ ] All refresh tokens from the same login share the same `family_id`
- [ ] On `/refresh`, the service checks if the family is already marked as revoked
- [ ] If a token from a family is reused, ALL tokens in that family are revoked
- [ ] Token reuse detection returns 401 with reason "family_revoked"
- [ ] Revoke only affects the compromised family -- other sessions remain valid
- [ ] Logout removes only the specific session's family tokens
- [ ] Redis data structures: `family:{family_id}` (set), `family:{family_id}:revoked` (bool)
- [ ] Metrics: `refresh_reuse_detected_total` counts family revocations

## Dependencies

- Depends on Story 3.1 (refresh token rotation)
- Intersects with Story 3.5 (logout-all implementation)

## Risk / Trade-offs

- **Family ID persistence**: The `family_id` must be stored with the refresh token in Redis. If the refresh token is compromised, the attacker can see the `family_id` (it's in the JWT or stored in Redis). This is not a security issue -- the family_id is a non-secret identifier.
- **Multiple sessions**: A user can have many active sessions. If the user logs out of all sessions (logout-all), all families must be revoked. This requires iterating over all families for the user, which is O(n) where n is the number of sessions. For most users, n is small (< 5).
- **Redis set size**: The `family:{family_id}` set can grow to the number of refreshes for a session. With 30-day refresh tokens and daily refreshes, the set has ~30 entries. This is small and fits efficiently in Redis.
