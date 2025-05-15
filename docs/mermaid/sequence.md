```mermaid
sequenceDiagram
    participant SPA as SPA
    participant Sesame as Sesame Auth Server
    participant Redis as Redis Cache
    participant PG as PostgreSQL+pgJWT
    participant Otel as OpenTelemetry

    Note over SPA,Sesame: 1. User’s refresh_token is in an HttpOnly cookie

    SPA->>Sesame: POST /token\ngrant_type=refresh_token
    activate Sesame

    Sesame->>Redis: 2. Validate refresh_token
    activate Redis
    Redis-->>Sesame: OK
    deactivate Redis

    Sesame->>PG: 3. (Optional) Load user record
    activate PG
    PG-->>Sesame: User data
    deactivate PG

    Sesame->>PG: 4. Generate new access & refresh JWTs via pgJWT
    activate PG
    PG-->>Sesame: { access_token, refresh_token }
    deactivate PG

    Sesame->>Redis: 5. Store rotated refresh_token
    activate Redis
    Redis-->>Sesame: Stored
    deactivate Redis

    Sesame-->>SPA: 6. 200 OK\n{ access_token, refresh_token }
    deactivate Sesame

    Note over Sesame,Otel: Instrumentation spans
    Sesame->>Otel: 7. Export refresh span


```


# Role Sequence

```mermaid
flowchart TD
    A[Org Admin Dashboard] --> B[Roles & Permissions Page]
    B --> C[Role List]
    C --> D[+ Create Role]
    C --> E[Click “Edit” on role]
D --> F[Create Role Form]
E --> G[Edit Role Form]
F & G --> H[Form Fields]

subgraph H
HF1[Name input]
HF2[Description input]
HF3[Permissions multi-select (checkbox list)]
HF4[Parent Roles multi-select]
HF5[Assigned Users multi-select]
HF6[Save / Cancel]
end

H --> I[On Save → call POST/PUT endpoints]
I --> C[Refresh Role List]


```

