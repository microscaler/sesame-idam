# Sesame-IDAM Docker

Same DX pattern as RERP. Microservice images are built from **Dockerfile.template** (rendered with service name). Binaries are cross-compiled on the host and copied into the image.

- **docker/microservices/Dockerfile.template** — Template for authentication and authorization services. Rendered with `--service authentication` or `--service authorization` (when tooling supports it) or use a simple placeholder Dockerfile per service.
- **Build flow:** `just gen` → build Rust binary → copy to build_artifacts/ → docker build (from template) → push to localhost:5001 or kind load.

When gen+impl exist, Tilt will drive build → copy-binary → build-image-simple (or equivalent) and deploy via Helm.
