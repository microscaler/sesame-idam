# sesame-idam-microservice

Helm chart for Sesame-IDAM microservices (authentication, authorization). Same pattern as RERP `rerp-microservice`.

- **values/authentication.yaml** — Authentication (Identity) service, port 8001.
- **values/authorization.yaml** — Authorization (Access Management) service, port 8002.

Deploy with Tilt when gen+impl crates exist; or manually:

```bash
helm upgrade --install authentication ./helm/sesame-idam-microservice -f ./helm/sesame-idam-microservice/values/authentication.yaml -n sesame-idam
helm upgrade --install authorization ./helm/sesame-idam-microservice -f ./helm/sesame-idam-microservice/values/authorization.yaml -n sesame-idam
```
