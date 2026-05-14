# Base runtime image for Sesame-IDAM microservices
# This image provides the minimal runtime environment for all sesame-idam services
# Services use this as a base and copy their pre-built binaries

ARG TARGETPLATFORM=linux/amd64
ARG BUILDPLATFORM
FROM --platform=${TARGETPLATFORM} alpine:3.23

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    libgcc \
    tzdata

# Create app directory with proper permissions for live updates
WORKDIR /app

# Create standard directories
RUN mkdir -p /app/config /app/doc /app/static_site && \
    chmod -R 777 /app

# Set timezone to UTC
ENV TZ=UTC

# Default environment variables
ENV RUST_BACKTRACE=1
ENV RUST_LOG=info

# Add dev synchronization script
COPY docker/base/dev-entrypoint.sh /dev-entrypoint.sh
RUN chmod +x /dev-entrypoint.sh

# Labels for identification
LABEL org.opencontainers.image.title="Sesame-IDAM Base Runtime"
LABEL org.opencontainers.image.description="Base runtime image for Sesame-IDAM microservices"
LABEL org.opencontainers.image.vendor="Microscaler"
