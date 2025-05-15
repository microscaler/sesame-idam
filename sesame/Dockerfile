FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y \
  libssl-dev \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY target/release/sesame /usr/local/bin/sesame

ENV HOST_PORT=0.0.0.0:3001

CMD ["sesame"]
