# Sesame-IDAM Development Environment
#
# Ports: Tilt UI 10351, identity-login 8001, authz-core 8002, api-keys 8003,
#        org-mgmt 8004, identity-session 8005, identity-user-mgmt 8006
#
# Prerequisites:
#   1. Kind cluster named 'sesame-idam' exists (create with: just dev-up)
#   2. Redis runs in namespace 'sesame-idam' (applied by this Tiltfile)
#   3. Supabase Postgres in namespace 'data' (apply with: just supabase-apply)
#
# Run: just dev-up (starts Tilt systemd service on port 10351)

allow_k8s_contexts(['kind-sesame-idam'])

update_settings(k8s_upsert_timeout_secs=60)
config.define_string('tilt_port', args=False, usage='Port for Tilt web UI')
cfg = config.parse()
tilt_port = cfg.get('tilt_port', '10351')
os.putenv('TILT_PORT', tilt_port)

# ====================
# Configuration
# ====================

# BRRTRouter checkout (sibling repo)
brrtrouter_root = '../BRRTRouter'
os.putenv('BRRTROUTER_ROOT', brrtrouter_root)

# Host architecture
host_machine = str(local('uname -m', quiet=True)).strip()
if host_machine in ['arm64', 'aarch64']:
    TARGET_ARCH_NAME = 'arm64'
    TARGET_RUST_TRIPLE = 'aarch64-unknown-linux-musl'
else:
    TARGET_ARCH_NAME = 'amd64'
    TARGET_RUST_TRIPLE = 'x86_64-unknown-linux-musl'

# Service definitions: name -> (spec_path, port, package_name, out_dir)
SERVICES = {
    'identity-login-service': ('openapi/identity-login-service/openapi.yaml', 8001, 'sesame_idam_identity_login_service_gen', 'microservices/idam/identity-login-service/gen'),
    'identity-session-service': ('openapi/identity-session-service/openapi.yaml', 8005, 'sesame_idam_identity_session_service_gen', 'microservices/idam/identity-session-service/gen'),
    'identity-user-mgmt-service': ('openapi/identity-user-mgmt-service/openapi.yaml', 8006, 'sesame_idam_identity_user_mgmt_service_gen', 'microservices/idam/identity-user-mgmt-service/gen'),
    'authz-core': ('openapi/authz-core/openapi.yaml', 8002, 'sesame_idam_authz_core_gen', 'microservices/idam/authz-core/gen'),
    'api-keys': ('openapi/api-keys/openapi.yaml', 8003, 'sesame_idam_api_keys_gen', 'microservices/idam/api-keys/gen'),
    'org-mgmt': ('openapi/org-mgmt/openapi.yaml', 8004, 'sesame_idam_org_mgmt_gen', 'microservices/idam/org-mgmt/gen'),
}

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
    '**/.ruff_cache',
    '**/.mypy_cache',
    '**/*.egg',
    '**/*.egg-info',
    '**/*.eggs',
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

# ====================
# Data components
# ====================
k8s_yaml('k8s/microservices/namespace.yaml')
k8s_yaml('k8s/data/persistent-volumes.yaml')
k8s_yaml('k8s/data/redis.yaml')

k8s_resource(
    'redis',
    port_forwards=['6379:6379'],
    labels=['data'],
)

# ====================
# Microservice helpers (following hauliage pattern)
# ====================

def create_microservice_lint(name, spec_file):
    """Lint a single OpenAPI spec via brrtrouter-gen."""
    local_resource(
        '%s-lint' % name,
        cmd='''
            set -e
            echo "🔍 Linting %s OpenAPI spec..."
            %s/target/debug/brrtrouter-gen lint \
                --spec ./openapi/%s \
                --fail-on-error || \
            cargo run --manifest-path %s/Cargo.toml --bin brrtrouter-gen -- \
                lint \
                --spec ./openapi/%s \
                --fail-on-error
            echo "✅ %s OpenAPI spec linting passed"
        ''' % (name, brrtrouter_root, spec_file, brrtrouter_root, spec_file, name),
        deps=[
            './openapi/%s' % spec_file,
        ],
        resource_deps=[],
        labels=[name],
        allow_parallel=True,
    )

def create_microservice_gen(name, spec_file, out_dir, package_name):
    """Run codegen for a single microservice."""
    # Build the path string for brrtrouter-gen generate command
    gen_cmd = (
        'cd %s && '
        'cargo run --bin brrtrouter-gen -- generate '
        '--spec "$(cd - >/dev/null && pwd)/openapi/%s" '
        '--output "$(cd - >/dev/null && pwd)/%s" '
        '--package-name %s '
        '--force'
    ) % (brrtrouter_root.replace('../', ''), spec_file, out_dir, package_name)
    
    local_resource(
        '%s-service-gen' % name,
        cmd=gen_cmd,
        deps=[
            './openapi/%s' % spec_file,
        ],
        ignore=[
            './microservices/idam/%s/gen/src' % name,
            './microservices/idam/%s/gen/doc' % name,
            './microservices/idam/%s/gen/config' % name,
        ],
        resource_deps=['%s-lint' % name],
        labels=[name],
        allow_parallel=True,
    )

def create_microservice_build_resource(name, package_name):
    """Build the gen crate binary."""
    local_resource(
        'build-%s' % name,
        'cd microservices && cargo build -p %s' % package_name,
        deps=[
            './microservices/idam/%s/gen/Cargo.toml' % name,
            './microservices/idam/%s/gen/src' % name,
            './microservices/Cargo.toml',
        ],
        ignore=[
            './microservices/target',
        ],
        resource_deps=['%s-service-gen' % name],
        labels=[name],
        allow_parallel=True,
    )

def create_microservice_deployment(name, port, package_name):
    """Create a Docker image + K8s Deployment + Service for a microservice."""
    image_name = 'localhost:5001/sesame-idam-%s' % name
    
    # Target path where cargo builds the binary
    target_path = 'microservices/target/%s/debug/%s' % (TARGET_RUST_TRIPLE, package_name)
    binary_name = name.replace('-', '_')
    artifact_path = 'build_artifacts/%s/%s' % (TARGET_ARCH_NAME, binary_name)
    hash_path = artifact_path + '.sha256'
    
    dockerfile_template = 'docker/microservices/Dockerfile.template'
    gen_doc_dir = 'microservices/idam/%s/gen/doc' % name
    gen_static_dir = 'microservices/idam/%s/gen/static_site' % name
    impl_config_dir = 'microservices/idam/%s/impl/config' % name
    
    # Step 1: Copy binary to artifacts
    local_resource(
        'copy-%s' % name,
        'mkdir -p build_artifacts/%s && cp %s %s' % (TARGET_ARCH_NAME, target_path, artifact_path),
        deps=[target_path],
        resource_deps=['build-%s' % name],
        labels=[name],
        allow_parallel=True,
    )
    
    # Step 2: Create SHA256 hash
    local_resource(
        'hash-%s' % name,
        'cd build_artifacts/%s && sha256sum %s > %s.sha256' % (TARGET_ARCH_NAME, binary_name, binary_name),
        deps=[artifact_path],
        resource_deps=['copy-%s' % name],
        labels=[name],
        allow_parallel=True,
    )
    
    # Step 3: Docker build with template rendering
    # The Dockerfile.template uses {{service_name}}, {{binary_name}}, {{port}}, {{module}}
    local_resource(
        'docker-%s' % name,
        'cp %s %s && '
        'sed -e "s/{{service_name}}/%s/g" -e "s/{{binary_name}}/%s/g" -e "s/{{port}}/%d/g" -e "s/{{module}}/%s/g" %s > /tmp/dockerfile-%s && '
        'docker build -f /tmp/dockerfile-%s -t %s:tilt . && '
        '(docker push %s:tilt 2>/dev/null || kind load docker-image %s:tilt --name sesame-idam)' % (
            dockerfile_template, artifact_path,
            name.replace('-', '_'), binary_name, port,
            name.replace('-', '_'), dockerfile_template,
            name, image_name, image_name, image_name
        ),
        deps=[hash_path, artifact_path, dockerfile_template],
        resource_deps=['hash-%s' % name],
        labels=[name],
        allow_parallel=False,
    )
    
    # Step 4: Custom build for Tilt live updates
    custom_build(
        image_name,
        'cp %s %s && '
        'sed -e "s/{{service_name}}/%s/g" -e "s/{{binary_name}}/%s/g" -e "s/{{port}}/%d/g" -e "s/{{module}}/%s/g" %s > /tmp/dockerfile-%s && '
        'docker build -f /tmp/dockerfile-%s -t %s:tilt . && '
        '(docker push %s:tilt 2>/dev/null || kind load docker-image %s:tilt --name sesame-idam)' % (
            dockerfile_template, artifact_path,
            name.replace('-', '_'), binary_name, port,
            name.replace('-', '_'), dockerfile_template,
            name, image_name, image_name, image_name
        ),
        deps=[artifact_path, hash_path, dockerfile_template, impl_config_dir, gen_doc_dir, gen_static_dir],
        tag='tilt',
        live_update=[
            sync(artifact_path, '/app/%s' % binary_name),
            sync(impl_config_dir, '/app/config/'),
            sync(gen_doc_dir, '/app/doc/'),
            sync(gen_static_dir, '/app/static_site/'),
            run('kill -HUP 1', trigger=[artifact_path]),
        ],
    )
    
    # Step 5: Helm deployment
    helm_values = [
        './helm/sesame-idam-microservice/values/%s.yaml' % name,
        './helm/sesame-idam-microservice/values/_database-kubernetes.yaml',
    ]
    k8s_yaml(helm('./helm/sesame-idam-microservice', name=name, namespace='sesame-idam', values=helm_values))
    
    # Step 6: Kubernetes resource configuration
    k8s_resource(
        name,
        port_forwards=['%s:%s' % (port, port)],
        resource_deps=['docker-%s' % name],
        labels=[name],
        auto_init=True,
        trigger_mode=TRIGGER_MODE_AUTO,
    )

# ====================
# Register all microservices
# ====================

for name in SERVICES:
    # out_dir is already in SERVICES[4-tuple]
    spec_file, port, package_name, out_dir = SERVICES[name]
    
    create_microservice_lint(name, spec_file)
    create_microservice_gen(name, spec_file, out_dir, package_name)
    create_microservice_build_resource(name, package_name)
    create_microservice_deployment(name, port, package_name)

# ====================
# Wait for all gens before starting deps
# ====================
local_resource(
    'sesame-all-gens',
    'echo "✅ All sesame codegen complete"',
    resource_deps=['%s-service-gen' % name for name in SERVICES],
    labels=['all_gens'],
    allow_parallel=False,
)
