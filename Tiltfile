# Sesame-IDAM Development Environment
#
# Tilt's UI port is fixed via --port flag. This repo uses 10351.
#   tilt up --port 10351 --host 0.0.0.0
# Full stack: `just dev-up` / `just dev-down` (same port).
#
# This Tiltfile orchestrates the build and deployment of all 6 Sesame-IDAM
# microservices using a base Docker image and pre-built binaries.

# ====================
# Configuration
# ====================

# Shared default cluster: context kind-kind (shared-kind-cluster).
allow_k8s_contexts(['kind-kind'])

update_settings(k8s_upsert_timeout_secs=60)

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

# Port mapping
IDAM_PORTS = {
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

DISCOVERED_SERVICES = SERVICE_NAMES
print("Sesame-IDAM Tilt discovered %d services" % len(DISCOVERED_SERVICES))

# ====================
# Package Names
# ====================
def get_package_name(name):
    """Return the impl crate package name for a service."""
    # These match the TARGET names from PRD Phase 1 (after naming fix)
    # For now, use the current (broken) names since Phase 1 hasn't run yet
    fallback = 'sesame_idam_' + name.replace('-', '_')
    manifest = 'microservices/idam/%s/impl/Cargo.toml' % name
    result = str(local('test -e "%s" && echo yes || true' % manifest, quiet=True)).strip()
    if result != 'yes':
        return fallback

    # Read package name from Cargo.toml using shell
    pkg = str(local('grep "^name = " "%s" | head -1 | sed "s/^name = *//;s/[^a-zA-Z0-9_-]//g"' % manifest, quiet=True)).strip()
    return pkg if pkg else fallback

def get_service_port(name):
    if name in IDAM_PORTS:
        return IDAM_PORTS[name]
    return '8100'

# ====================
# Helper Functions
# ====================

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

def create_microservice_build(name, package_name):
    """Build a service using the sesame-idam CLI shim."""
    local_resource(
        'build-%s' % name,
        cmd='%s build microservice %s' % (sesame_idam_bin, name),
        deps=[
            './microservices/idam/%s/gen/Cargo.toml' % name,
            './microservices/idam/%s/impl/Cargo.toml' % name,
            './microservices/idam/%s/gen/src' % name,
            './microservices/idam/%s/impl/src' % name,
            'tooling/pyproject.toml',
        ],
        ignore=[
            './microservices/target',
            './build_artifacts',
        ],
        resource_deps=['%s-service-gen' % name],
        labels=[name],
        allow_parallel=True,
    )

def create_microservice_deployment(name, port):
    """Create Docker image and k8s deployment for a service."""
    package_name = get_package_name(name)
    binary_name = package_name
    target_path = 'microservices/target/x86_64-unknown-linux-musl/debug/%s' % package_name
    artifact_path = 'build_artifacts/%s' % binary_name
    image_name = 'localhost:5001/sesame-idam-%s' % name

    # Copy binary to build_artifacts
    local_resource(
        'copy-%s' % name,
        cmd='%s docker copy-binary %s %s %s' % (sesame_idam_bin, target_path, artifact_path, binary_name),
        deps=[target_path, 'tooling/pyproject.toml'],
        resource_deps=['build-%s' % name],
        labels=[name],
        allow_parallel=True,
    )

    # Build Docker image
    local_resource(
        'docker-%s' % name,
        cmd='%s docker build-image-simple %s build_artifacts/%s %s --service %s' % (
            sesame_idam_bin, image_name, binary_name, 'docker/microservices/Dockerfile.template', name
        ),
        deps=[
            artifact_path,
            'docker/microservices/Dockerfile.template',
            'docker/base/Dockerfile',
            'tooling/pyproject.toml',
        ],
        resource_deps=['copy-%s' % name],
        labels=[name],
        allow_parallel=True,
    )

    # Helm chart values per service
    helm_values = 'helm/sesame-idam-microservice/values/%s.yaml' % name

    # Deploy via Helm
    k8s_yaml(
        helm('helm/sesame-idam-microservice', name=name, namespace=namespace,
             values=[helm_values]),
    )

    # Port forward and link to docker resource
    k8s_resource(
        name,
        port_forwards=['%s:%s' % (port, port)],
        resource_deps=['docker-%s' % name],
        labels=['sesame-idam_' + name],
        auto_init=True,
    )

# ====================
# Data Infrastructure
# ====================
# Redis and PostgreSQL are managed by shared-kind-cluster's Tilt.
# Do NOT stand up data infrastructure here — let the shared cluster own it.

# ====================
# Per-Service Resources
# ====================
for name in DISCOVERED_SERVICES:
    port = get_service_port(name)
    spec_path = IDAM_SPEC_PATHS.get(name, '%s/openapi.yaml' % name)
    package_name = get_package_name(name)

    print("Sesame-IDAM Tilt: configuring service '%s' (port %s, package %s)" % (name, port, package_name))

    # Full build pipeline: lint -> gen -> build -> copy -> docker -> k8s
    create_microservice_lint(name, spec_path)
    create_microservice_gen(name, spec_path)
    create_microservice_build(name, package_name)
    create_microservice_deployment(name, port)
