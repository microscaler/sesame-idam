# saml-verifier (ADR-014 §4, amended 2026-10-11): the one Sesame image with C XML libraries in it.
#
# Its own Cargo workspace (microservices/saml-verifier), built and tested here, never by the Rust buildpack, so
# libxmlsec1, libxml2 and OpenSSL stay out of the suite. Base images are pinned by digest; the Debian packages come
# from that release and are listed into the image (/app/packages.txt) rather than pinned to versions that Debian
# security updates would remove from the mirrors. A samael, libxmlsec1 or libxml2 change is a pull request that
# passes the whole test suite in the build stage.
ARG RUST_IMAGE=docker.io/library/rust:1.99-slim-bookworm@sha256:2c3a22f0a5533ea2dd5a16627bc841228151faa2d4de2644ac9987e4a2f1f2fa
ARG RUNTIME_IMAGE=docker.io/library/debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587

FROM ${RUST_IMAGE} AS build
RUN apt-get update \
 && apt-get install -y --no-install-recommends make pkg-config clang libclang-dev libxmlsec1-dev libxml2-dev libssl-dev \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY microservices/saml-verifier /src
# The tests sign responses with throwaway keys at run time: a failing rule fails the image build.
RUN cargo test --workspace --locked \
 && cargo build --release --locked --bin saml-verifier

FROM ${RUNTIME_IMAGE}
RUN apt-get update \
 && apt-get install -y --no-install-recommends libxmlsec1 libxmlsec1-openssl libxml2 libxslt1.1 libssl3 \
 && dpkg-query -W -f '${Package} ${Version}\n' > /tmp/packages.txt \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /src/target/release/saml-verifier /app/saml-verifier
COPY microservices/saml-verifier/gen/doc /app/doc
COPY microservices/saml-verifier/impl/config /app/config
RUN mv /tmp/packages.txt /app/packages.txt
# Non-root, nothing writable needed (the chart sets a read-only root file system).
USER 65532:65532
ENV PORT=8080
EXPOSE 8080
ENTRYPOINT ["/app/saml-verifier", "--spec", "/app/doc/openapi.yaml", "--config", "/app/config/config.yaml", "--doc-dir", "/app/doc"]
