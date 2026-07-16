"""One-shot helper to emit Flux HelmRelease stubs for the idam suite profile."""

from __future__ import annotations

from pathlib import Path

SERVICES: list[tuple[str, str, dict]] = [
    (
        "identity-login-service",
        "sesame-idam-identity-login-service",
        {
            "authzCoreUrl": "http://authz-core:8080",
            "jwks": True,
        },
    ),
    (
        "identity-session-service",
        "sesame-idam-identity-session-service",
        {"jwks": True},
    ),
    ("identity-user-mgmt-service", "sesame-idam-identity-user-mgmt-service", {}),
    ("authz-core", "sesame-idam-authz-core", {}),
    ("api-keys", "sesame-idam-api-keys", {}),
    ("org-mgmt", "sesame-idam-org-mgmt", {}),
]

JWKS_URL = (
    "http://identity-session-service.sesame-idam.svc.cluster.local:8080"
    "/idam/v1/.well-known/jwks.json"
)


def _config_block(extra: dict) -> str:
    if not extra:
        return ""
    lines = ["      config:"]
    if "authzCoreUrl" in extra:
        lines.append(f"        authzCoreUrl: \"{extra['authzCoreUrl']}\"")
    if extra.get("jwks"):
        lines.extend(
            [
                "        security:",
                "          jwks:",
                "            BearerAuth:",
                f"              jwks_url: \"{JWKS_URL}\"",
                '              iss: "https://idam.example.com"',
                '              aud: "sesame-idam"',
            ]
        )
    return "\n".join(lines) + "\n"


def main() -> None:
    root = Path(__file__).resolve().parents[3] / (
        "deployment-configuration/profiles/dev/sesame-idam/idam/services"
    )
    root.mkdir(parents=True, exist_ok=True)
    resources: list[str] = []
    for svc, image, extra in SERVICES:
        marker = f'# {{"$imagepolicy": "flux-system:{image}:tag"}}'
        body = f"""apiVersion: helm.toolkit.fluxcd.io/v2
kind: HelmRelease
metadata:
  name: {svc}
spec:
  interval: 5m
  timeout: 5m
  releaseName: {svc}
  targetNamespace: sesame-idam
  install:
    remediation:
      retries: 3
  upgrade:
    remediation:
      retries: 3
      remediateLastFailure: true
  chart:
    spec:
      chart: ./helm/sesame-idam-microservice
      reconcileStrategy: Revision
      sourceRef:
        kind: GitRepository
        name: product-sesame-idam
        namespace: flux-system
      interval: 1m
  valuesFrom:
    - kind: ConfigMap
      name: sesame-idam-idam-common-helm-values
      valuesKey: values.yaml
  values:
    service:
      name: {svc}
    image:
      name: {image}
      tag: dev-0 {marker}
    app:
      binaryName: {svc}
      serviceName: {svc}
{_config_block(extra)}"""
        (root / f"{svc}.yaml").write_text(body)
        resources.append(f"  - {svc}.yaml")

    (root / "kustomization.yaml").write_text(
        f"""apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

namespace: sesame-idam

resources:
{chr(10).join(resources)}

configMapGenerator:
  - name: sesame-idam-idam-common-helm-values
    files:
      - values.yaml=values/common.yaml

generatorOptions:
  disableNameSuffixHash: true
  immutable: false
"""
    )
    print(f"wrote {len(resources)} HelmReleases under {root}")


if __name__ == "__main__":
    main()
