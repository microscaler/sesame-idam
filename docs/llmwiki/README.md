# Sesame-IDAM LLM Wiki

This is the living, LLM-maintained knowledge base for Sesame-IDAM.

## Start Here

1. **[`SCHEMA.md`](./SCHEMA.md)** — Page conventions, status tags, source-of-truth order.
2. **[`index.md`](./index.md)** — Full content catalog organized by category. Read this to find relevant pages for your task.
3. **[`log.md`](./log.md)** — Recent session activity and agent updates.

## Quick Navigation

| Task | Read These |
|------|-----------|
| Understanding service architecture | [`topics/topic-architecture-overview.md`](./topics/topic-architecture-overview.md) |
| How login works | [`topics/topic-login-flow.md`](./topics/topic-login-flow.md) |
| How authorization works | [`topics/topic-authorization-flow.md`](./topics/topic-authorization-flow.md) |
| Data model | [`topics/topic-data-model.md`](./topics/topic-data-model.md) |
| JWT schema | [`topics/topic-jwt-schema.md`](./topics/topic-jwt-schema.md) |
| Codegen conventions | [`topics/topic-brrtrouter-codegen.md`](./topics/topic-brrtrouter-codegen.md) |
| Entity definitions | [`entities/`](./entities/) directory |

## Key Principle

Read `index.md`, pick only pages whose title or heading matches your task. **Never load the entire wiki into context.** The wiki contains 25+ files — you will never need all of them in a single session.
