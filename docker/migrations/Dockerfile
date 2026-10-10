# Sesame-IDAM's schema as a release artifact: the Lifeguard migration SQL (migrations/, apply_order.txt order, RLS
# contract included) and the seeds, applied by scripts/sesame-migrate.sh. Runs as the migration Job before every
# release (chart workload.kind job, Helm pre-install/pre-upgrade hook). Dev keeps scripts/setup-db.sh (Tilt).
ARG POSTGRES_IMAGE=docker.io/library/postgres:17-bookworm

FROM ${POSTGRES_IMAGE} AS seeds
COPY microservices/idam /src/microservices/idam
RUN mkdir -p /out && cd /src && find microservices -path '*/impl/seeds/*.sql' -exec cp --parents {} /out/ \;

FROM ${POSTGRES_IMAGE}
COPY migrations /sesame/migrations
COPY microservices/idam/seed_order.txt microservices/idam/seed_environments.txt /sesame/microservices/idam/
COPY --from=seeds /out/microservices /sesame/microservices
COPY --chmod=0755 scripts/sesame-migrate.sh /usr/local/bin/sesame-migrate
USER postgres
WORKDIR /sesame
ENTRYPOINT ["/usr/local/bin/sesame-migrate"]
