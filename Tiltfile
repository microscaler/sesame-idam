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

DISCOVERED_SERVICES = SERVICE_NAMES
print("Sesame-IDAM Tilt discovered %d services" % len(DISCOVERED_SERVICES))

# ====================
# Helper Functions
# ====================

def get_service_port(name):
    if name in IDAM_PORTS:
        return IDAM_PORTS[name]
    return '8100'

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
    """Create Docker image and k8s deployment for a service.

    Matches hauliage pattern exactly:
    1. copy-binary: binary -> build_artifacts/<arch>/<name> + SHA256
    2. build-image-simple: render Dockerfile.template, build image
    3. custom_build: Tilt live_update with sync + kill -HUP reload
    4. k8s_yaml(helm): Helm deployment
    5. k8s_resource: port forward + labels + deps
    """
    # Package name — used to find the Cargo-built binary and matches what
    # render_dockerfile_template() resolves via get_binary_names() (which is
    # monkey-patched by sesame_idam_tooling to read [[bin]] from Cargo.toml).
    package_name = PACKAGE_NAMES.get(name, 'sesame_idam_' + name.replace('-', '_'))
    binary_name = name.replace('-', '_')

    # Paths — both target (Cargo output) and artifacts (copy-binary output +
    # Dockerfile COPY) must use package_name since that's what the template
    # resolves via the monkey-patched get_binary_names().
    target_path = 'microservices/target/%s/debug/%s' % (TARGET_RUST_TRIPLE, package_name)
    artifact_path = 'build_artifacts/%s/%s' % (TARGET_ARCH_NAME, package_name)
    hash_path = 'build_artifacts/%s/%s.sha256' % (TARGET_ARCH_NAME, package_name)
    dockerfile_template = 'docker/microservices/Dockerfile.template'
    image_name = 'localhost:5001/sesame-idam-%s' % name

    # 1. Copy binary from workspace build to artifacts and create SHA256 hash
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

    # 2. Build and push Docker image (template rendered on the fly with --service)
    # CLI signature: build-image-simple <image> <dockerfile_template> <hash_path> <artifact_path> --service <name>
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

    # 3. Custom build for Tilt live updates
    # Ensures image exists (build if custom_build runs before docker-%s),
    # then push to localhost:5001 or kind load.
    custom_build(
        image_name,
        ('%s docker build-image-simple %s %s %s %s --service %s' % (
            sesame_idam_bin, image_name, dockerfile_template, hash_path, artifact_path, name
        ) + ' && (docker push %s:tilt 2>/dev/null || kind load docker-image %s:tilt --name sesame-idam)' % (image_name, image_name)),
        deps=[dockerfile_template,
              'microservices/idam/%s/impl/config' % name,
              'microservices/idam/%s/gen/doc' % name,
              'microservices/idam/%s/gen/static_site' % name],
        resource_deps=['build-%s' % name],
        tag='tilt',
        live_update=[
            sync(artifact_path, '/app/%s' % package_name),
            sync('microservices/idam/%s/impl/config/' % name, '/app/config/'),
            sync('microservices/idam/%s/gen/doc/' % name, '/app/doc/'),
            sync('microservices/idam/%s/gen/static_site/' % name, '/app/static_site/'),
            run('kill -HUP 1', trigger=[artifact_path]),
        ],
    )

    # 4. Deploy using Helm
    helm_values = ['helm/sesame-idam-microservice/values/%s.yaml' % name]
    k8s_yaml(
        helm('helm/sesame-idam-microservice', name=name, namespace=namespace, values=helm_values),
    )

    # 5. Kubernetes resource configuration
    k8s_resource(
        name,
        port_forwards=['%s:%s' % (port, port)],
        resource_deps=['docker-%s' % name],
        labels=[name],
        auto_init=True,
        trigger_mode=TRIGGER_MODE_AUTO,
    )

# ====================
# Data Infrastructure
# ====================
# Create the namespace so Helm manifests have a target.
# Redis and PostgreSQL are managed by shared-kind-cluster's Tilt.
# Do NOT stand up data infrastructure here — let the shared cluster own it.
k8s_yaml('k8s/microservices/namespace.yaml')

# ====================
# Per-Service Resources
# ====================
for name in DISCOVERED_SERVICES:
    port = get_service_port(name)
    spec_path = IDAM_SPEC_PATHS.get(name, '%s/openapi.yaml' % name)

    print("Sesame-IDAM Tilt: configuring service '%s' (port %s)" % (name, port))

    # Full build pipeline: lint -> gen -> build -> copy -> docker -> k8s
    create_microservice_lint(name, spec_path)
    create_microservice_gen(name, spec_path)
    create_microservice_build(name)
    create_microservice_deployment(name, port)
