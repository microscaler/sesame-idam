# Sesame Auth.js reference

Support tier: Reference

This is a thin, conformance-tested Auth.js provider preset. It does not
implement OAuth or JWT validation. Auth.js discovers Sesame from the issuer and
owns Authorization Code, PKCE, state, nonce, callback, and ID-token validation.

```ts
import NextAuth from "next-auth"
import { Sesame } from "./src/sesame.js"

export const { handlers, auth, signIn, signOut } = NextAuth({
  providers: [
    Sesame({
      issuer: process.env.AUTH_SESAME_ISSUER!,
      clientId: process.env.AUTH_SESAME_ID!,
      clientSecret: process.env.AUTH_SESAME_SECRET,
    }),
  ],
})
```

Register the exact Auth.js callback URI in Sesame. Public clients omit the
secret; confidential clients register `client_secret_basic`. Store refresh
tokens in server-side Auth.js state, never browser local storage. Provider
logout and organization switching remain explicit application journeys rather
than being conflated with local Auth.js session deletion.

Run `npm test` and `npm run typecheck` on ms02.
