[private]
default:
  @just --list --unsorted


generate:
  cargo run --bin crdgen > yaml/doc_crds/crd.yaml
  helm template --release-name 'tilt' charts/yair-controller > yaml/deployment.yaml
  cat yaml/deployment.yaml


# run without opentelemetry
run:
  RUST_LOG=info,kube=debug cargo run

# format with nightly rustfmt
fmt:
  cargo +nightly fmt --all -- --check


# format with nightly rustfmt
clippy:
  cargo clippy --all-features -- -D warnings -W clippy::pedantic -W clippy::nursery -W rust-2018-idioms

# run unit tests
test:
  cargo test

# run integration tests
test-integration: install-crd
  cargo test -- --ignored
# run telemetry tests
test-telemetry:
  OPENTELEMETRY_ENDPOINT_URL=http://127.0.0.1:4317 cargo test --all-features -- get_trace_id_returns_valid_traces --ignored

## compile for musl (for docker image)
#compile features="telemetry":
#  rm -f yair-controller-amd64 yair-controller-darwin || true
#  docker run --rm -t \
#  --mount type=bind,source=$(pwd),target=/volume \
#  --mount type=bind,source=$HOME/.cargo/registry,target=/root/.cargo/registry \
#  --mount type=bind,source=$HOME/.cargo/git,target=/root/.cargo/git \
#  clux/muslrust:nightly \
#  cargo build --release --features={{features}} --bin yair-controller
#  cp target/aarch64-unknown-linux-musl/release/yair-controller ./yair-controller-amd64
#  cp target/release/yair-controller ./yair-controller-darwin || true

#package: compile
#  docker buildx build --platform linux/amd64,linux/arm64 -t casibbald/yair-controller:local .

#[private]
#_build features="":
#  just compile {{features}}
#  docker buildx build --platform linux/amd64,linux/arm64 -t casibbald/yair-controller:local .


#
## local helper for test-telemetry and run-telemetry
## forward grpc otel port from svc/promstack-tempo in monitoring
#forward-tempo:
#  kubectl port-forward -n monitoring svc/promstack-tempo 55680:4317