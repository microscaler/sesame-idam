# Role/database bootstrap image for Sesame-IDAM.
# Application migrations are intentionally excluded (Tilt owns that cycle).
ARG POSTGRES_IMAGE=docker.io/bitnamilegacy/postgresql:17.6.0-debian-12-r2
FROM ${POSTGRES_IMAGE}

USER 0
COPY --chmod=0755 scripts/db-init-job.sh /usr/local/bin/sesame-idam-db-init
USER 1001

ENTRYPOINT ["/usr/local/bin/sesame-idam-db-init"]
