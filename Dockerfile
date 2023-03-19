# instructions for small build sizes: 
# https://kerkour.com/rust-small-docker-image
# https://shaneutt.com/blog/rust-fast-small-docker-image-builds/
# https://levelup.gitconnected.com/create-an-optimized-rust-alpine-docker-image-1940db638a6c
# https://www.youtube.com/watch?v=xuqolj01D7M

# TODO access to report



# ###############################################
# Mult step builder image

### PREPARE ###
FROM rust:1.66.0 as planner
WORKDIR /app
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


### BUILD DEPENDENCIES ###
FROM rust:1.66.0 as cacher
WORKDIR /app
# RUN apt update && apt install lld clang -y 
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json


### BUILD BURL ###
FROM rust:1.66.0 AS builder

RUN update-ca-certificates

WORKDIR /app
# copy dependencies
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/target
COPY . .
RUN cargo build --release --bin burl-cli
RUN strip -s /app/target/release/burl-cli


### RUNTIME STAGE ###
FROM debian:bullseye-slim AS runtime
# use this base since we want to provide nano, bash and other basic bins

LABEL git = "https://github.com/s-weil/burl"
LABEL version = "0.0.1"

WORKDIR /app

COPY --from=builder /app/target/release/burl-cli burl-cli

RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # for later possibility to update the.toml file
    && apt-get install -y nano \
    # Clean up and remove unnecessary
    && apt-get autoremove -y \
    && apt-get clean -y \
    # && rm -rf /var/lib/apt/lists/*
    && rm -rf /var/lib/apt/ \
    && rm -rf /var/lib/dpkg/

# add some helpers
# COPY --from=busybox:1.35.0-uclibc /bin/sh /bin/sh
# COPY --from=busybox:1.35.0-uclibc /bin/ls /bin/ls

# for user specific use cases
RUN mkdir -p ./config
RUN mkdir -p ./data
COPY /examples/actix/specs.toml ./config

ENTRYPOINT [ "/bin/bash", "-c", "./burl-cli" ]
CMD [ "help" ]
