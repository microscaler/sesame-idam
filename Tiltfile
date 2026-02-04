# Sesame-IDAM Development Environment (same DX as RERP)
# Run with: tilt up
# Cluster: kind-sesame-idam (create with: just dev-up)

allow_k8s_contexts(['kind-sesame-idam'])

update_settings(k8s_upsert_timeout_secs=60)
config.define_string('tilt_port', args=False, usage='Port for Tilt web UI')
cfg = config.parse()
tilt_port = cfg.get('tilt_port', '10351')
os.putenv('TILT_PORT', tilt_port)

# ====================
# Tooling (sesame CLI)
# ====================
TOOLING_IGNORE = [
    '**/*.pyc',
    '**/*.pyo',
    '**/__pycache__',
    '**/.pytest_cache',
    '**/.coverage',
    '**/.coverage.*',
    '**/htmlcov',
    '**/coverage.xml',
    '**/.ruff_cache',
    '**/.mypy_cache',
    '**/*.egg',
    '**/*.egg-info',
    '**/.eggs',
    '**/dist',
    '**/.hypothesis',
]

local_resource(
    'build-tooling',
    'just build-tooling',
    deps=[
        './tooling/src',
        './tooling/pyproject.toml',
    ],
    ignore=TOOLING_IGNORE,
    labels=['tooling'],
    allow_parallel=True,
)

local_resource(
    'lint-tooling',
    'just lint-fix && just format',
    deps=[
        './tooling/src',
        './tooling/tests',
        './tooling/pyproject.toml',
    ],
    ignore=TOOLING_IGNORE,
    labels=['tooling'],
    allow_parallel=True,
)

local_resource(
    'test-tooling',
    'tooling/.venv/bin/pytest tooling/tests -v --tb=short',
    deps=[
        './tooling/src',
        './tooling/tests',
        './tooling/pyproject.toml',
    ],
    ignore=TOOLING_IGNORE,
    labels=['tooling'],
    allow_parallel=True,
)

# ====================
# Data components (Redis; Supabase Postgres externalised to microscaler-supabase)
# ====================
# Supabase stack (namespace data, postgres, etc.) is applied via: just supabase-apply
# (requires microscaler-supabase as side-clone at ../microscaler-supabase).
# Tilt loads: sesame-idam namespace, Redis PV, Redis.

k8s_yaml('k8s/microservices/namespace.yaml')
k8s_yaml('k8s/data/persistent-volumes.yaml')
k8s_yaml('k8s/data/redis.yaml')

k8s_resource(
    'redis',
    port_forwards=['6379:6379'],
    labels=['data'],
)

# ====================
# IDAM Microservices (when gen+impl exist)
# ====================
# When microservices/idam/authentication/gen and impl (and authorization) exist:
# - Add create_microservice_lint(name, spec) for openapi/idam/authentication/openapi.yaml etc.
# - Add create_microservice_gen(name, spec, output_dir) calling just gen-auth / just gen-authorization or sesame gen
# - Add create_microservice_build_resource(name) and create_microservice_deployment(name)
# - Use docker/microservices/Dockerfile.template, helm/sesame-idam-microservice, build_artifacts/amd64/
# - Ports: authentication 8001, authorization 8002 (see helm values/)
# For now only tooling is live; run just gen to generate gen crates, then add impl and wire Tilt here.
