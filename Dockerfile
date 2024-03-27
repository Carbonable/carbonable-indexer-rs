FROM rust:1.74-slim-bookworm as builder
RUN set -eux; \
  export DEBIAN_FRONTEND=noninteractive; \
  echo "deb http://deb.debian.org/debian sid main" >> /etc/apt/sources.list; \
  apt update; \
  apt install --yes pkg-config ca-certificates openssl libssl-dev protobuf-compiler curl unzip; \
  apt clean autoclean; \
  apt autoremove --yes; \
  rm -rf /var/lib/apt/* /var/lib/dpkg/* /var/lib/cache/* /var/lib/log/*; \
  echo "Installed base utils!"

WORKDIR /srv/www
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/usr/local/cargo/git \
  --mount=type=cache,target=/usr/local/rustup \
  set -eux; \
  rustup install stable; \
  cargo build --release; \
  objcopy --compress-debug-sections target/release/carbonable-api ./carbonable-api; \
  objcopy --compress-debug-sections target/release/carbonable-indexer ./carbonable-indexer; \
  objcopy --compress-debug-sections target/release/carbonable-migration ./carbonable-migration

FROM debian:bookworm-slim as production-runtime

RUN set -eux; \
  export DEBIAN_FRONTEND=noninteractive; \
  echo "deb http://deb.debian.org/debian sid main" >> /etc/apt/sources.list; \
  apt update; \
  apt install --yes pkg-config ca-certificates openssl libssl-dev protobuf-compiler; \
  apt clean autoclean; \
  apt autoremove --yes; \
  rm -rf /var/lib/apt/* /var/lib/dpkg/* /var/lib/cache/* /var/lib/log/*; \
  echo "Installed base utils!"

WORKDIR /srv/www

COPY --from=builder /srv/www/data ./data
COPY --from=builder /srv/www/carbonable-api ./carbonable-api
COPY --from=builder /srv/www/carbonable-indexer ./carbonable-indexer
COPY --from=builder /srv/www/carbonable-migration ./carbonable-migration

CMD ["./carbonable-api"]
