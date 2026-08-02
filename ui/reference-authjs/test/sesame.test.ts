import { describe, expect, it } from "vitest"

import { Sesame } from "../src/sesame.js"

describe("Sesame Auth.js provider", () => {
  it("uses issuer discovery and all OIDC checks", () => {
    const provider = Sesame({
      issuer: "https://id.sesameidentity.dev.local",
      clientId: "fixture-public-client",
    })

    expect(provider.type).toBe("oidc")
    expect(provider.issuer).toBe("https://id.sesameidentity.dev.local")
    expect(provider.checks).toEqual(["pkce", "state", "nonce"])
    expect(provider.authorization?.params?.scope).toBe("openid profile email")
  })

  it("maps a standards-compliant UserInfo profile", () => {
    const provider = Sesame({
      issuer: "https://id.sesameidentity.dev.local",
      clientId: "fixture-public-client",
    })

    expect(
      provider.profile?.({
        sub: "user-123",
        email: "user@example.com",
        email_verified: true,
        name: "Example User",
      }, {}),
    ).toEqual({
      id: "user-123",
      email: "user@example.com",
      name: "Example User",
      image: null,
    })
  })
})
