# Sesame-IDAM Development Environment
#
# Tilt's UI port is fixed via --port flag. This repo uses 10351.
#   tilt up --port 10351 --host 0.0.0.0
# Full stack: `just dev-up` / `just dev-down` (same port).
#
# GitOps (rerp pattern): on shared-k8s, Flux owns HelmReleases + runtime
# ConfigMaps/Secrets + DB bootstrap Jobs. Tilt builds and publishes
# registry images (dev-<nanoseconds>) only — never helm()/k8s_yaml for
# microservices. Kind/local still applies Helm when FLUX_OWNS_DEPLOY=0.
# Application migrations stay Tilt-owned (`sesame-idam-apply-migrations`).

# ====================
# Configuration
# ====================

_SHARED_K8S_KCFG = os.path.abspath('../shared-k8s-cluster/kubeconfig/shared-k8s.yaml')
_SHARED_K8S_REGISTRY = '10.177.76.220:5000'
_k8s_mode = os.environ.get('TILT_K8S_CLUSTER', '').strip().lower()
if _k8s_mode in ('kind', 'kind-kind'):
    _use_shared_k8s = False
elif _k8s_mode in ('shared-k8s', 'k3s'):
    _use_shared_k8s = True
else:
    _use_shared_k8s = os.path.exists(_SHARED_K8S_KCFG)

if _use_shared_k8s and os.path.exists(_SHARED_K8S_KCFG):
    allow_k8s_contexts(['shared-k8s'])
    os.putenv('KUBECONFIG', _SHARED_K8S_KCFG)
    default_registry(_SHARED_K8S_REGISTRY)
else:
    allow_k8s_contexts(['kind-kind'])

# When true: Flux product-components owns Helm/Secrets; Tilt publishes images only.
# Default ON for shared-k8s (matches rerp); Kind defaults OFF unless overridden.
_flux_owns_default = '1' if _use_shared_k8s else '0'
FLUX_OWNS_DEPLOY = os.environ.get('FLUX_OWNS_DEPLOY', _flux_owns_default).strip() in (
    '1', 'true', 'TRUE', 'yes',
)
print('Sesame-IDAM Tilt: FLUX_OWNS_DEPLOY=%s (shared-k8s=%s)' % (
    FLUX_OWNS_DEPLOY, _use_shared_k8s,
))

update_settings(k8s_upsert_timeout_secs=60)

# Rust/cargo on ms02 (rustup) — Tilt local_resource cmd does not load login shells.
RUST_ENV_PREFIX = 'export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH" && '

# Configure automatic Docker pruning to prevent disk space exhaustion
docker_prune_settings(
    disable=False,
    max_age_mins=30,
    keep_recent=1,
    interval_hrs=1
)

# BRRTRouter checkout for brrtrouter-gen. Override:
#   export BRRTROUTER_ROOT=/path/to/BRRTRouter
_brrtrouter_env = os.getenv('BRRTROUTER_ROOT', '').strip().rstrip('/')
brrtrouter_root = _brrtrouter_env if _brrtrouter_env else '../BRRTRouter'
os.putenv('BRRTROUTER_ROOT', str(local('realpath "%s"' % brrtrouter_root, quiet=True)).strip())

# Shared Python env for brrtrouter + sesame-idam CLIs. Override:
#   export BRRTROUTER_VENV=/path/to/venv
_brrtrouter_venv_env = os.getenv('BRRTROUTER_VENV', '').strip().rstrip('/')
_home = os.getenv('HOME', '') or os.getenv('USERPROFILE', '')
brrtrouter_venv = _brrtrouter_venv_env if _brrtrouter_venv_env else (_home + '/.local/share/brrtrouter/venv' if _home else '.local/share/brrtrouter/venv')
sesame_idam_bin = '%s/bin/sesame-idam' % brrtrouter_venv

# Namespace
namespace = 'sesame-idam'

# Data stack: postgres/redis managed by shared-k8s-cluster tilt, not this stack
bundled_data_stack = False

# ====================
# Dynamic Architecture Detection
# ====================
# Mirrors hauliage pattern: detect host arch at Tilt startup time.
# Must be at module level (outside functions) so it's evaluated once.
host_machine = str(local('uname -m', quiet=True)).strip()
if host_machine in ['arm64', 'aarch64']:
    TARGET_ARCH_NAME = 'arm64'
    TARGET_RUST_TRIPLE = 'aarch64-unknown-linux-musl'
else:
    TARGET_ARCH_NAME = 'amd64'
    TARGET_RUST_TRIPLE = 'x86_64-unknown-linux-musl'

print("Sesame-IDAM Tilt: detected arch=%s target=%s" % (TARGET_ARCH_NAME, TARGET_RUST_TRIPLE))

# ====================
# Tooling
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
    '**/*.egg',
    '**/*.egg-info',
    '**/*.egg-info/**',
    '**/brrtrouter_tooling.egg-info/**',
    '**/sesame_idam_tooling.egg-info/**',
    '**/*tooling.egg-info',
    '**/*tooling.egg-info/**',
    '**/.eggs',
    '**/dist',
    '**/build',
    '**/build/**',
    '**/.hypothesis',
    '**/.DS_Store',
]

# Tilt UI labels — strict one label per resource (never multi-label).
#   tooling | docker | data | database | migrations | testing | dev-tools | <service>
local_resource(
    'build-tooling',
    '''set -e
VENV="%s"
BRRROOT="%s"
test -d "$VENV" || python3 -m venv "$VENV"
"$VENV/bin/pip" install -U pip
"$VENV/bin/pip" install -e "$BRRROOT/tooling[dev]"
cd tooling && "$VENV/bin/pip" install -e '.[dev]'
''' % (brrtrouter_venv, brrtrouter_root),
    deps=[
        './tooling/pyproject.toml',
        '%s/tooling/pyproject.toml' % brrtrouter_root,
    ],
    ignore=TOOLING_IGNORE,
    labels=['tooling'],
    allow_parallel=True,
)

# ====================
# Base Docker Image
# ====================
local_resource(
    'build-base-image',
    '%s docker build-base' % sesame_idam_bin,
    deps=[
        'docker/base/Dockerfile',
        'docker/base/dev-entrypoint.sh',
    ],
    labels=['docker'],
    allow_parallel=True,
)

# ====================
# Services & Ports
# ====================
# Hardcoded service list (avoids Tilt blob/Starlark string parsing issues).
# Mirrors openapi/idam/bff-suite-config.yaml contents.

SERVICE_NAMES = [
    'identity-login-service',
    'identity-session-service',
    'identity-user-mgmt-service',
    'authz-core',
    'api-keys',
    'org-mgmt',
]

# Cluster-wide HTTP port (PRD: all app Services listen on 8080 in-cluster).
SERVICE_HTTP_PORT = '8080'

# Optional host port-forwards for isolated Sesame debugging (host:container).
# Container listens on 8080; familiar host ports map legacy dev scripts/smoke tests.
HOST_PORT_FORWARDS = {
    'identity-login-service': '8101:8080',
    'identity-session-service': '8105:8080',
}

# Legacy reference — service identity is Kubernetes Service name, not port number.
IDAM_LEGACY_PORTS = {
    'identity-login-service': '8101',
    'identity-session-service': '8105',
    'identity-user-mgmt-service': '8106',
    'authz-core': '8102',
    'api-keys': '8103',
    'org-mgmt': '8104',
}

# OpenAPI spec paths relative to openapi/idam/
IDAM_SPEC_PATHS = {
    'identity-login-service': 'identity-login-service',
    'identity-session-service': 'identity-session-service',
    'identity-user-mgmt-service': 'identity-user-mgmt-service',
    'authz-core': 'authz-core',
    'api-keys': 'api-keys',
    'org-mgmt': 'org-mgmt',
}

# Package name mapping (after Phase 1 naming fix).
# Must match [package].name in each impl/Cargo.toml.
PACKAGE_NAMES = {
    'identity-login-service': 'sesame_idam_identity_login_service',
    'identity-session-service': 'sesame_idam_identity_session_service',
    'identity-user-mgmt-service': 'sesame_idam_identity_user_mgmt_service',
    'authz-core': 'sesame_idam_authz_core',
    'api-keys': 'sesame_idam_api_keys',
    'org-mgmt': 'sesame_idam_org_mgmt',
}

# just gen-* recipe names (differs from service dir names for identity-* services).
_GEN_JUST_NAMES = {
    'identity-login-service': 'gen-identity-login',
    'identity-session-service': 'gen-identity-session',
    'identity-user-mgmt-service': 'gen-identity-user-mgmt',
    'authz-core': 'gen-authz-core',
    'api-keys': 'gen-api-keys',
    'org-mgmt': 'gen-org-mgmt',
}

DISCOVERED_SERVICES = SERVICE_NAMES
print("Sesame-IDAM Tilt discovered %d services" % len(DISCOVERED_SERVICES))

# Desktop dev test defaults (ms02 LAN Postgres + Redis). Override via OS/Tilt env.
_TEST_DB_HOST = os.environ.get('TEST_DB_HOST', '192.168.1.189')
_TEST_DB_PORT = os.environ.get('TEST_DB_PORT', '5433')
_TEST_REDIS_URL = os.environ.get('TEST_REDIS_URL', 'redis://192.168.1.189:6390')
_TEST_SSOREADY_URL = os.environ.get('SESAME_SSOREADY_API_URL', 'http://127.0.0.1:9190')
_TEST_DB_PASS = os.environ.get('TEST_DB_PASS', 'dev_password_change_in_prod')
_TEST_DATABASE_URL = 'postgres://sesame_idam:%s@%s:%s/sesame_idam' % (
    _TEST_DB_PASS,
    _TEST_DB_HOST,
    _TEST_DB_PORT,
)

# Prefix for manual test local_resources (export before cargo/just).
TEST_ENV_SHELL = (
    'export TEST_DB_HOST="%s" && '
    + 'export TEST_DB_PORT="%s" && '
    + 'export TEST_REDIS_URL="%s" && '
    + 'export SESAME_SSOREADY_API_URL="%s" && '
    + 'export DATABASE_URL="%s" && '
    + 'export TEST_DATABASE_URL="%s"'
) % (
    _TEST_DB_HOST,
    _TEST_DB_PORT,
    _TEST_REDIS_URL,
    _TEST_SSOREADY_URL,
    _TEST_DATABASE_URL,
    _TEST_DATABASE_URL,
)

_COMMON_TEST_DEPS = [
    './microservices/Cargo.toml',
    './justfile',
    './.config/nextest.toml',
]

# ====================
# Helper Functions
# ====================

def get_service_port(name):
    """In-cluster HTTP port (always 8080 after k8s-native migration)."""
    return SERVICE_HTTP_PORT

def create_microservice_lint(name, spec_file):
    """Lint an OpenAPI spec with brrtrouter-gen."""
    local_resource(
        '%s-lint' % name,
        cmd='''set -e
echo "Linting %s OpenAPI spec..."
%s/target/debug/brrtrouter-gen lint \
    --spec ./openapi/idam/%s/openapi.yaml \
    --fail-on-error 2>/dev/null || \
cargo run --manifest-path %s/Cargo.toml --bin brrtrouter-gen -- \
    lint \
    --spec ./openapi/idam/%s/openapi.yaml \
    --fail-on-error
echo "OK %s OpenAPI spec linting passed"
''' % (name, brrtrouter_root, spec_file, brrtrouter_root, spec_file, name),
        deps=[
            './openapi/idam/%s/openapi.yaml' % spec_file,
        ],
        resource_deps=[],
        labels=[name],
        allow_parallel=True,
    )

def create_microservice_gen(name, spec_file):
    """Generate code for a service using the sesame-idam CLI shim."""
    local_resource(
        '%s-service-gen' % name,
        cmd='%s gen suite idam --service %s' % (sesame_idam_bin, name),
        deps=[
            './openapi/idam/%s/openapi.yaml' % spec_file,
            'tooling/pyproject.toml',
        ],
        ignore=[
            './microservices/idam/%s/gen/src' % name,
            './microservices/idam/%s/gen/doc' % name,
            './microservices/idam/%s/impl/config' % name,
            './microservices/idam/%s/gen/static_site' % name,
        ],
        resource_deps=['%s-lint' % name],
        labels=[name],
        allow_parallel=True,
    )

def create_microservice_build(name):
    """Build a service using the sesame-idam CLI shim."""
    local_resource(
        'build-%s' % name,
        cmd='%s build microservice %s' % (sesame_idam_bin, name),
        deps=[
            './microservices/idam/%s/gen/Cargo.toml' % name,
            './microservices/idam/%s/impl/Cargo.toml' % name,
            './microservices/idam/%s/gen/src' % name,
            './microservices/idam/%s/impl/src' % name,
            '%s/src' % brrtrouter_root,
            'tooling/pyproject.toml',
        ],
        ignore=[
            './microservices/target',
            './build_artifacts',
        ],
        resource_deps=['%s-service-gen' % name, 'build-tooling'],
        labels=[name],
        allow_parallel=True,
    )

def create_microservice_deployment(name, port):
    """Build images for a service; deploy via Flux (shared-k8s) or Tilt Helm (Kind).

    Shared-k8s / FLUX_OWNS_DEPLOY (rerp pattern):
      copy → docker → image-* push (dev-<ns>). Flux HelmRelease owns Deployment + ConfigMap.

    Kind / FLUX_OWNS_DEPLOY=0:
      copy → docker → custom_build (live_update) → helm() + k8s_resource (includes *-config CM).
    """
    package_name = PACKAGE_NAMES.get(name, 'sesame_idam_' + name.replace('-', '_'))

    target_path = 'microservices/target/%s/debug/%s' % (TARGET_RUST_TRIPLE, package_name)
    artifact_path = 'build_artifacts/%s/%s' % (TARGET_ARCH_NAME, package_name)
    hash_path = 'build_artifacts/%s/%s.sha256' % (TARGET_ARCH_NAME, package_name)
    dockerfile_template = 'docker/microservices/Dockerfile.template'
    # Flux ImageRepository expects sesame-idam-<svc> (no localhost_5001_ rewrite).
    image_repo = 'sesame-idam-%s' % name
    image_name = image_repo if FLUX_OWNS_DEPLOY else 'localhost:5001/%s' % image_repo

    local_resource(
        'copy-%s' % name,
        '%s docker copy-binary %s %s %s' % (
            sesame_idam_bin, target_path, artifact_path, package_name
        ),
        deps=['tooling/pyproject.toml'],
        resource_deps=['build-%s' % name],
        labels=[name],
        allow_parallel=True,
    )

    if FLUX_OWNS_DEPLOY:
        # local_resource (not custom_build): Tilt drops custom_build targets that are
        # not referenced by a Kubernetes manifest. Flux consumes the pushed tags.
        registry_image = '%s/%s' % (_SHARED_K8S_REGISTRY, image_repo)
        local_resource(
            'image-%s' % image_repo,
            '''set -eu
%s docker build-image-simple %s %s %s %s --service %s
DEV_REF="%s:dev-$(date +%%s%%N)"
docker tag %s:tilt "$DEV_REF"
docker push "$DEV_REF"
echo "Published $DEV_REF for Flux image discovery"
''' % (
                sesame_idam_bin,
                image_name,
                dockerfile_template,
                hash_path,
                artifact_path,
                name,
                registry_image,
                image_name,
            ),
            deps=[
                artifact_path,
                hash_path,
                dockerfile_template,
                'microservices/idam/%s/impl/config' % name,
                'microservices/idam/%s/gen/doc' % name,
                'microservices/idam/%s/gen/static_site' % name,
                'tooling/pyproject.toml',
            ],
            resource_deps=['build-base-image', 'copy-%s' % name],
            labels=[name],
            allow_parallel=True,
        )
    else:
        local_resource(
            'docker-%s' % name,
            '%s docker build-image-simple %s %s %s %s --service %s' % (
                sesame_idam_bin, image_name, dockerfile_template, hash_path, artifact_path, name
            ),
            deps=[dockerfile_template, 'tooling/pyproject.toml'],
            resource_deps=['build-base-image', 'copy-%s' % name],
            labels=[name],
            allow_parallel=False,
        )
        custom_build(
            image_name,
            ('%s docker build-image-simple %s %s %s %s --service %s' % (
                sesame_idam_bin, image_name, dockerfile_template, hash_path, artifact_path, name
            ) + ' && (docker push %s:tilt 2>/dev/null || kind load docker-image %s:tilt --name sesame-idam)' % (image_name, image_name)),
            deps=[
                dockerfile_template,
                'build_artifacts',
                'microservices/idam/%s/impl/config' % name,
                'microservices/idam/%s/gen/doc' % name,
                'microservices/idam/%s/gen/static_site' % name,
                'microservices/idam/%s/impl/src' % name,
                'microservices/idam/%s/gen/src' % name,
            ],
            tag='tilt',
            live_update=[
                sync(artifact_path, '/app/%s' % package_name),
                sync('microservices/idam/%s/impl/config/' % name, '/app/config/'),
                sync('microservices/idam/%s/gen/doc/' % name, '/app/doc/'),
                sync('microservices/idam/%s/gen/static_site/' % name, '/app/static_site/'),
                run('kill -HUP 1', trigger=[artifact_path]),
            ],
        )

        helm_values = [
            'helm/sesame-idam-microservice/values/%s.yaml' % name,
            'helm/sesame-idam-microservice/values/_http-kubernetes.yaml',
            'helm/sesame-idam-microservice/values/_database-kubernetes.yaml',
        ]
        k8s_yaml(
            helm('helm/sesame-idam-microservice', name=name, namespace=namespace, values=helm_values),
        )

        # Keep Helm ConfigMap with the Deployment (missing CM → MountVolume fail).
        _port_forwards = [HOST_PORT_FORWARDS[name]] if name in HOST_PORT_FORWARDS else []
        k8s_resource(
            name,
            port_forwards=_port_forwards,
            objects=['%s-config:configmap:%s' % (name, namespace)],
            resource_deps=['sesame-idam-database-env', 'docker-%s' % name],
            labels=[name],
            auto_init=True,
            trigger_mode=TRIGGER_MODE_AUTO,
        )

def _bdd_impl_deps(name):
    return [
        './microservices/idam/%s/impl/src' % name,
        './microservices/idam/%s/impl/tests' % name,
    ]

def create_manual_bdd_test(name):
    """Per-service BDD (main_bdd) against LAN Postgres — manual Tilt trigger."""
    package = PACKAGE_NAMES[name]
    local_resource(
        'test-bdd-%s' % name,
        RUST_ENV_PREFIX + '''set -e
echo "=== BDD: %s ==="
%s
cd microservices
cargo nextest run -p %s --test main_bdd --test-threads 1 --fail-fast
''' % (name, TEST_ENV_SHELL, package),
        deps=_COMMON_TEST_DEPS + _bdd_impl_deps(name),
        ignore=['./microservices/target'],
        labels=['testing'],
        trigger_mode=TRIGGER_MODE_MANUAL,
        auto_init=False,
        allow_parallel=False,
    )

# ====================
# Data Infrastructure
# ====================
# Create the namespace so Helm manifests have a target.
# Redis and PostgreSQL are managed by shared-k8s-cluster's Tilt.
# Do NOT stand up data infrastructure here — let the shared cluster own it.
# When FLUX_OWNS_DEPLOY=1, runtime ConfigMap/Secret come from product profiles.
if not FLUX_OWNS_DEPLOY:
    k8s_yaml('k8s/microservices/namespace.yaml')
    k8s_yaml('k8s/microservices/database-env.yaml')
    k8s_resource(
        new_name='sesame-idam-database-env',
        objects=[
            'sesame-idam-database-config:configmap:sesame-idam',
            'sesame-idam-db-credentials:secret:sesame-idam',
        ],
        labels=['data'],
    )

# Redis: shared platform instance in namespace `data`
# (redis.data.svc.cluster.local:6379, managed by shared-k8s-cluster).
# No app-local Redis — do not duplicate the data stack here.

# ====================
# Per-Service Resources
# ====================

# ====================
# Database Init & Migration (rerp split)
# ====================
# Flux owns role/database/schema bootstrap via Job image sesame-idam-db-init
# (scripts/db-init-job.sh). Tilt publishes that image and applies migrations only.
# Never put Lifeguard SQL / seeds into the Flux Job.
_apply_migrations_deps = ['postgres'] if bundled_data_stack else []

# Role/database bootstrap image. Flux runs this as a gated Job before reconciling
# service HelmReleases. Tilt only publishes the content-addressed bootstrap image.
DB_INIT_IMAGE = 'sesame-idam-db-init'
DB_INIT_DOCKERFILE = 'docker/jobs/Dockerfile'
DB_INIT_REF = '%s/%s' % (_SHARED_K8S_REGISTRY, DB_INIT_IMAGE)
DB_INIT_BUILD = '''set -eu
docker build -f %s -t %s:tilt .
DEV_REF="%s:dev-$(date +%%s%%N)"
docker tag %s:tilt "$DEV_REF"
docker push "$DEV_REF"
echo "Published $DEV_REF for Flux image discovery"
''' % (DB_INIT_DOCKERFILE, DB_INIT_IMAGE, DB_INIT_REF, DB_INIT_IMAGE)
local_resource(
    'image-%s' % DB_INIT_IMAGE,
    DB_INIT_BUILD,
    deps=[
        DB_INIT_DOCKERFILE,
        'scripts/db-init-job.sh',
    ],
    labels=['database'],
    allow_parallel=True,
)

# Break-glass / non-Flux Kind: full role+DB+migrations via kubectl exec.
# Prefer Flux Job + sesame-idam-apply-migrations when FLUX_OWNS_DEPLOY=1.
if not FLUX_OWNS_DEPLOY:
    local_resource(
        'sesame-idam-db-init',
        'chmod +x ./scripts/setup-db.sh && ./scripts/setup-db.sh',
        deps=['./scripts/setup-db.sh'],
        resource_deps=['postgres'] if bundled_data_stack else [],
        labels=['database'],
        trigger_mode=TRIGGER_MODE_MANUAL,
        auto_init=True,
    )

# Ad-hoc: regenerate SQL under migrations/ + apply_order.txt from Lifeguard
# entity registries. Does NOT connect to PostgreSQL — only writes files.
local_resource(
    'sesame-idam-migrate',
    'cd microservices && cargo run -p sesame_idam_migrator',
    deps=[
        './microservices/migrator',
        './microservices/idam/identity-login-service/impl/src/models',
        './microservices/idam/identity-session-service/impl/src/models',
        './microservices/idam/identity-user-mgmt-service/impl/src/models',
        './microservices/idam/authz-core/impl/src/models',
        './microservices/idam/api-keys/impl/src/models',
        './microservices/idam/org-mgmt/impl/src/models',
        './microservices/Cargo.toml',
    ],
    ignore=['./microservices/target'],
    labels=['migrations'],
    trigger_mode=TRIGGER_MODE_MANUAL,
    auto_init=False,
    allow_parallel=True,
)

# Rapid-development application migration cycle. Trigger after Flux
# sesame-idam-idam foundation is Ready (role/DB/schema exist). Applies only
# migrations/RLS/seeds/grants — role/database/schema stay Flux-owned.
local_resource(
    'sesame-idam-apply-migrations',
    'chmod +x ./scripts/setup-db.sh && SESAME_IDAM_APPLY_MIGRATIONS_ONLY=1 ./scripts/setup-db.sh',
    deps=[
        './scripts/setup-db.sh',
        './migrations',
        './microservices/idam/identity-login-service/impl/seeds',
        './microservices/idam/identity-session-service/impl/seeds',
        './microservices/idam/identity-user-mgmt-service/impl/seeds',
        './microservices/idam/authz-core/impl/seeds',
        './microservices/idam/api-keys/impl/seeds',
        './microservices/idam/org-mgmt/impl/seeds',
    ],
    resource_deps=_apply_migrations_deps,
    labels=['migrations'],
    trigger_mode=TRIGGER_MODE_MANUAL,
    auto_init=False,
    allow_parallel=False,
)

# ====================
# Manual test hooks (Tilt UI or `just tilt-trigger <resource>`)
# ====================
# ms02 defaults: TEST_DB_HOST=192.168.1.189 TEST_DB_PORT=5433, broker :9190 (k8s PF).
# Remote Mac trigger: tilt trigger <resource> --host 192.168.1.189 --port 10351
#
# Suite resources (labels: testing):
#   sesame-idam-test-saml-proof     — Track A gate (lint + gen sync + broker + SAML BDD)
#   sesame-idam-test-openapi-lint   — lint all 6 OpenAPI specs
#   sesame-idam-test-openapi-sync   — cmp canonical specs vs gen/doc (drift detector)
#   sesame-idam-test-pact-broker      — pact-mock-server contract tests
#   sesame-idam-test-nt-fast          — `just nt` (workspace nextest, no db_integration_suite)
#   sesame-idam-test-nt-db-suite      — serial db_integration_suite only
#   sesame-idam-test-bdd-all          — main_bdd for all 6 services (serial)
#   test-bdd-<service>                — single-service BDD (one per DISCOVERED_SERVICES)

_openapi_sync_checks = '\n'.join([
    (
        'cmp -s openapi/idam/%s/openapi.yaml microservices/idam/%s/gen/doc/openapi.yaml '
        + '|| { echo "❌ %s gen/doc/openapi.yaml drift — run: just %s"; exit 1; }'
    ) % (IDAM_SPEC_PATHS[name], name, name, _GEN_JUST_NAMES[name])
    for name in DISCOVERED_SERVICES
])

_bdd_all_steps = '\n'.join([
    (
        'echo "=== BDD: %s ===" && cargo nextest run -p %s --test main_bdd --test-threads 1 --fail-fast'
    ) % (name, PACKAGE_NAMES[name])
    for name in DISCOVERED_SERVICES
])

_all_bdd_deps = _COMMON_TEST_DEPS
for _bdd_name in DISCOVERED_SERVICES:
    _all_bdd_deps = _all_bdd_deps + _bdd_impl_deps(_bdd_name)

local_resource(
    'sesame-idam-test-saml-proof',
    '''set -e
echo "=== Track A SAML proof suite ==="
%s
just saml-proof-suite
''' % TEST_ENV_SHELL,
    deps=_COMMON_TEST_DEPS + [
        './openapi/idam/org-mgmt/openapi.yaml',
        './openapi/idam/identity-login-service/openapi.yaml',
        './microservices/pact-mock-server/tests',
        './microservices/idam/org-mgmt/impl/tests/bdd',
        './microservices/idam/identity-login-service/impl/tests/bdd',
        './microservices/database/src/saml_proof.rs',
    ],
    ignore=['./microservices/target'],
    resource_deps=['sesame-idam-broker'],
    labels=['testing'],
    trigger_mode=TRIGGER_MODE_MANUAL,
    auto_init=False,
    allow_parallel=False,
)

local_resource(
    'sesame-idam-test-openapi-sync',
    '''set -e
echo "=== OpenAPI gen/doc sync (all 6 services) ==="
%s
''' % _openapi_sync_checks,
    deps=['./openapi/idam'] + [
        './microservices/idam/%s/gen/doc/openapi.yaml' % name
        for name in DISCOVERED_SERVICES
    ],
    labels=['testing'],
    trigger_mode=TRIGGER_MODE_MANUAL,
    auto_init=False,
    allow_parallel=True,
)

local_resource(
    'sesame-idam-test-openapi-lint',
    '''set -e
echo "=== OpenAPI lint (all 6 services) ==="
just lint-openapi
''',
    deps=['./openapi/idam'] + _COMMON_TEST_DEPS,
    labels=['testing'],
    trigger_mode=TRIGGER_MODE_MANUAL,
    auto_init=False,
    allow_parallel=True,
)

local_resource(
    'sesame-idam-test-pact-broker',
    RUST_ENV_PREFIX + '''set -e
echo "=== pact-mock-server (broker + contracts) ==="
%s
cd microservices
cargo nextest run -p pact-mock-server --test-threads 1 --fail-fast
''' % TEST_ENV_SHELL,
    deps=_COMMON_TEST_DEPS + [
        './microservices/pact-mock-server/src',
        './microservices/pact-mock-server/tests',
        './microservices/pact-mock-server/pacts',
    ],
    ignore=['./microservices/target'],
    resource_deps=['sesame-idam-broker'],
    labels=['testing'],
    trigger_mode=TRIGGER_MODE_MANUAL,
    auto_init=False,
    allow_parallel=False,
)

local_resource(
    'sesame-idam-test-nt-fast',
    RUST_ENV_PREFIX + '''set -e
echo "=== Workspace nextest (fast — excludes db_integration_suite) ==="
%s
just nt
''' % TEST_ENV_SHELL,
    deps=_COMMON_TEST_DEPS + ['./microservices'],
    ignore=['./microservices/target'],
    labels=['testing'],
    trigger_mode=TRIGGER_MODE_MANUAL,
    auto_init=False,
    allow_parallel=False,
)

local_resource(
    'sesame-idam-test-nt-db-suite',
    RUST_ENV_PREFIX + '''set -e
echo "=== DB integration suite (serial profile) ==="
%s
just nt-db-suite
''' % TEST_ENV_SHELL,
    deps=_COMMON_TEST_DEPS + ['./microservices'],
    ignore=['./microservices/target'],
    labels=['testing'],
    trigger_mode=TRIGGER_MODE_MANUAL,
    auto_init=False,
    allow_parallel=False,
)

local_resource(
    'sesame-idam-test-bdd-all',
    RUST_ENV_PREFIX + '''set -e
echo "=== BDD all services (main_bdd, serial) ==="
%s
cd microservices
%s
echo "✅ All service BDD suites passed"
''' % (TEST_ENV_SHELL, _bdd_all_steps),
    deps=_all_bdd_deps,
    ignore=['./microservices/target'],
    labels=['testing'],
    trigger_mode=TRIGGER_MODE_MANUAL,
    auto_init=False,
    allow_parallel=False,
)

for _test_svc in DISCOVERED_SERVICES:
    create_manual_bdd_test(_test_svc)

# ====================
# Pact broker dev tooling (SAML/OAuth mocks + contract publish)
# ====================
# Shared platform pact-broker lives in namespace `data` (shared-k8s-cluster Tilt).
# Sesame adds:
#   - sesame-idam-broker: SSOReady-compatible SAML + Google/Microsoft OAuth mocks (:9190)
#   - sesame-pact-manager: publishes pacts/*.json to pact-broker.data.svc.cluster.local:9292
#
# Override from host:
#   SESAME_BROKER_PORT              — host broker listen port (default 9190)
#   SESAME_BROKER_BASE_URL          — URLs returned to identity-login-service
#   SESAME_BROKER_APP_REDIRECT_URL  — post-SAML app callback (Loadlinker)
_PACT_MOCK_DIR = './microservices/pact-mock-server'
_PACT_MOCK_DEPS = [
    _PACT_MOCK_DIR + '/src',
    _PACT_MOCK_DIR + '/Cargo.toml',
    _PACT_MOCK_DIR + '/pacts',
]
_dev_registry = 'localhost:5001'

_sesame_broker_image = '%s/sesame-idam-broker' % _dev_registry
if _use_shared_k8s and os.path.exists(_SHARED_K8S_KCFG):
    _sesame_broker_push = 'docker tag %s:tilt $EXPECTED_REF && docker push $EXPECTED_REF' % _sesame_broker_image
else:
    _sesame_broker_push = '(docker push %s:tilt 2>/dev/null || kind load docker-image %s:tilt --name sesame-idam)' % (_sesame_broker_image, _sesame_broker_image)

custom_build(
    _sesame_broker_image,
    'docker build -f docker/microservices/Dockerfile.sesame_idam_broker -t %s:tilt . && %s' % (_sesame_broker_image, _sesame_broker_push),
    deps=_PACT_MOCK_DEPS + ['./docker/microservices/Dockerfile.sesame_idam_broker'],
    tag='tilt',
)

k8s_yaml('k8s/microservices/sesame-idam-broker.yaml')
k8s_resource(
    'sesame-idam-broker',
    port_forwards=['9190:9190'],
    resource_deps=[],
    labels=['dev-tools'],
)

# Pact contracts ConfigMap (regenerated when pacts/*.json change).
_cmd_pact_configmap = (
    'kubectl create configmap sesame-pact-contracts -n sesame-idam '
    + '--from-file=Sesame-SSO-Broker.json=' + _PACT_MOCK_DIR + '/pacts/Sesame-SSO-Broker.json '
    + '--from-file=Sesame-OAuth-Google.json=' + _PACT_MOCK_DIR + '/pacts/Sesame-OAuth-Google.json '
    + '--from-file=Sesame-OAuth-Microsoft.json=' + _PACT_MOCK_DIR + '/pacts/Sesame-OAuth-Microsoft.json '
    + '--dry-run=client -o yaml'
)
k8s_yaml(local(_cmd_pact_configmap))

_sesame_pact_manager_image = '%s/sesame-pact-manager' % _dev_registry
if _use_shared_k8s and os.path.exists(_SHARED_K8S_KCFG):
    _sesame_pact_manager_push = 'docker tag %s:tilt $EXPECTED_REF && docker push $EXPECTED_REF' % _sesame_pact_manager_image
else:
    _sesame_pact_manager_push = '(docker push %s:tilt 2>/dev/null || kind load docker-image %s:tilt --name sesame-idam)' % (_sesame_pact_manager_image, _sesame_pact_manager_image)

custom_build(
    _sesame_pact_manager_image,
    'docker build -f docker/microservices/Dockerfile.pact_manager -t %s:tilt . && %s' % (_sesame_pact_manager_image, _sesame_pact_manager_push),
    deps=_PACT_MOCK_DEPS + ['./docker/microservices/Dockerfile.pact_manager'],
    tag='tilt',
)

k8s_yaml('k8s/microservices/pact-broker-env.yaml')
k8s_yaml('k8s/microservices/pact-manager.yaml')
k8s_resource(
    new_name='sesame-pact-contracts',
    objects=['sesame-pact-contracts:configmap:sesame-idam'],
    labels=['dev-tools'],
)
k8s_resource(
    'sesame-pact-manager',
    port_forwards=['1238:1238'],
    resource_deps=['sesame-pact-contracts'],
    labels=['dev-tools'],
)

# Fast host loop for broker code changes (optional; cluster broker is primary for login-service).
# Default host port 9191 avoids clashing with k8s port-forward 9190:9190.
_sesame_broker_host_port = os.environ.get('SESAME_BROKER_HOST_PORT', '9191')
_sesame_broker_base = os.environ.get('SESAME_BROKER_BASE_URL', 'http://127.0.0.1:%s' % _sesame_broker_host_port)
_sesame_broker_redirect = os.environ.get(
    'SESAME_BROKER_APP_REDIRECT_URL',
    'http://loadlinker.dev.microscaler.local/saml/callback',
)

local_resource(
    'sesame-idam-broker-build',
    RUST_ENV_PREFIX + 'cd microservices && cargo build -p pact-mock-server --bin sesame-idam-broker',
    deps=_PACT_MOCK_DEPS + ['./microservices/Cargo.toml'],
    ignore=['./microservices/target'],
    labels=['dev-tools'],
    allow_parallel=True,
)

_sesame_broker_serve_cmd = (
    RUST_ENV_PREFIX
    + 'cd microservices && '
    + 'SESAME_BROKER_PORT=' + _sesame_broker_host_port + ' '
    + 'SESAME_BROKER_BASE_URL=' + _sesame_broker_base + ' '
    + 'SESAME_BROKER_APP_REDIRECT_URL=' + _sesame_broker_redirect + ' '
    + 'cargo run -p pact-mock-server --bin sesame-idam-broker'
)

local_resource(
    'sesame-idam-broker-host',
    serve_cmd=_sesame_broker_serve_cmd,
    resource_deps=['sesame-idam-broker-build'],
    deps=_PACT_MOCK_DEPS,
    labels=['dev-tools'],
    auto_init=False,
    trigger_mode=TRIGGER_MODE_MANUAL,
    allow_parallel=True,
)

# ====================
# Per-Service Resources
# ====================
for name in DISCOVERED_SERVICES:
    port = get_service_port(name)
    spec_path = IDAM_SPEC_PATHS.get(name, '%s/openapi.yaml' % name)

    print("Sesame-IDAM Tilt: configuring service '%s' (in-cluster :%s)" % (name, port))

    # Full build pipeline: lint -> gen -> build -> copy -> docker -> k8s
    create_microservice_lint(name, spec_path)
    create_microservice_gen(name, spec_path)
    create_microservice_build(name)
    create_microservice_deployment(name, port)
