# Usage default features:
# tilt up
#
# Usage with features:
# tilt up telemetry
config.define_string("features", args=True)
cfg = config.parse()


features = cfg.get('features', "")
print("compiling with features: {}".format(features))

load('ext://configmap', 'configmap_create')
k8s_yaml('./yaml/services/collector/collector.yaml')
k8s_yaml('./yaml/services/prometheus/prometheus.yaml')
k8s_yaml('./yaml/services/prometheus/config.yaml')
k8s_yaml('./yaml/services/grafana/grafana.yaml')
configmap_create('tilt-grafana-config',
                 from_file=[
                   'grafana.ini=./yaml/services/grafana/grafana.ini',
                   'dashboards.yaml=./yaml/services/grafana/dashboards.yaml',
                   'datasource-prometheus.yaml=./yaml/services/grafana/datasource-prometheus.yaml',
                 ])
configmap_create('tilt-grafana-dashboards',
                 from_file=[
                   'tiltfile-execution.json=./yaml/services/grafana/tiltfile-execution.json',
                 ])

k8s_resource('tilt-local-metrics-collector',
             port_forwards=[
               port_forward(4317, name='receiverGrpc'),
               port_forward(8888),
             ],
             links=[
               link('http://localhost:8888/metrics', 'metrics'),
             ])


k8s_resource('tilt-local-metrics-prometheus',
             port_forwards=[
               port_forward(10353, 9090, name='prometheus'),
             ],
             resource_deps=['tilt-local-metrics-collector'])

k8s_resource('tilt-local-metrics-grafana',
             port_forwards=[
               port_forward(10354, 3000, name='grafana'),
             ],
             resource_deps=['tilt-local-metrics-prometheus'])

# experimental_metrics_settings(
#  enabled=True, address='localhost:8888', insecure=True, reporting_period='5s')


local_resource('fmt', 'just fmt')
local_resource('Pedantic as Fuck', 'just clippy')
local_resource('compile', 'just compile %s' % features)
local_resource('unit-test', 'just test-unit')
local_resource('test-telemetry', 'just test-telemetry')
docker_build('casibbald/yair-controller', '.', dockerfile='Dockerfile')
# local_resource('cleanup', 'just cleanup-resources')
local_resource('generate', 'just generate')
k8s_yaml('yaml/doc_crds/crd.yaml')
k8s_yaml('yaml/doc_crds/instance-samuel.yaml')
k8s_yaml('yaml/doc_crds/instance-lorem.yaml')
k8s_yaml('yaml/doc_crds/instance-illegal.yaml')
k8s_yaml('yaml/deployment.yaml')
k8s_resource('yair-controller', port_forwards=8080,)