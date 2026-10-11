# Sesame-IDAM's schema as a release artifact: the Lifeguard migration SQL (migrations/, apply_order.txt order, RLS
# contract included) and the seeds every environment gets (seed_environments.txt), applied by
# scripts/sesame-migrate.sh. Demo seeds (seed_order.txt: demo tenants, users, roles) never enter this image. Runs as the migration Job before every
# release (chart workload.kind job, Helm pre-install/pre-upgrade hook). Dev keeps scripts/setup-db.sh (Tilt).
ARG POSTGRES_IMAGE=docker.io/library/postgres:17-bookworm

FROM ${POSTGRES_IMAGE} AS seeds
COPY microservices/idam /src/microservices/idam
# Only the seeds listed in seed_environments.txt (comments and blank lines skipped); a listed file that is missing
# fails the build.
RUN mkdir -p /out/microservices/idam && cd /src/microservices/idam \
 && grep -v -e '^#' -e '^[[:space:]]*$' seed_environments.txt | while IFS= read -r f; do \
      cp --parents "$f" /out/microservices/idam/ || exit 1; \
    done

FROM ${POSTGRES_IMAGE}
COPY migrations /sesame/migrations
COPY microservices/idam/seed_environments.txt /sesame/microservices/idam/
COPY --from=seeds /out/microservices /sesame/microservices
COPY --chmod=0755 scripts/sesame-migrate.sh /usr/local/bin/sesame-migrate
USER postgres
WORKDIR /sesame
ENTRYPOINT ["/usr/local/bin/sesame-migrate"]
