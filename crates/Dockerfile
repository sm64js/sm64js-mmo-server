FROM debian:buster-slim as build

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    RUST_BACKTRACE=full

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    gcc \
    libc6-dev \
    wget \
    pkg-config \
    openssl \
    libssl-dev \
    curl \
    libpq-dev \
    ; \
    \
    url="https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init"; \
    wget "$url"; \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path --default-toolchain nightly-2022-01-06 --profile minimal; \
    rm rustup-init; \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME; \
    rustup --version; \
    cargo --version; \
    rustc --version; \
    \
    apt-get remove -y --auto-remove \
    wget \
    ; \
    rm -rf /var/lib/apt/lists/*;

# set up project
RUN cd / && \
    mkdir -p sm64js && \
    USER=root cargo init --bin sm64js
WORKDIR /sm64js

# copy files for dependency compilation
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./crates/sm64js/Cargo.toml ./crates/sm64js/Cargo.toml
COPY ./crates/sm64js-api/Cargo.toml ./crates/sm64js-api/Cargo.toml
COPY ./crates/sm64js-auth/Cargo.toml ./crates/sm64js-auth/Cargo.toml
COPY ./crates/sm64js-common/Cargo.toml ./crates/sm64js-common/Cargo.toml
COPY ./crates/sm64js-db/Cargo.toml ./crates/sm64js-db/Cargo.toml
COPY ./crates/sm64js-env/Cargo.toml ./crates/sm64js-env/Cargo.toml
COPY ./crates/sm64js-proto/Cargo.toml ./crates/sm64js-proto/Cargo.toml
COPY ./crates/sm64js-ws/Cargo.toml ./crates/sm64js-ws/Cargo.toml
RUN rm ./src/main.rs && \
    mkdir -p ./crates/sm64js/src && \
    echo "fn main() {}" >> ./crates/sm64js/src/main.rs && \
    mkdir -p ./crates/sm64js/benches && \
    echo "fn main() {}" >> ./crates/sm64js/benches/game.rs && \
    mkdir -p ./crates/sm64js-api/src && \
    touch ./crates/sm64js-api/src/lib.rs && \
    mkdir -p ./crates/sm64js-auth/src && \
    touch ./crates/sm64js-auth/src/lib.rs && \
    mkdir -p ./crates/sm64js-common/src && \
    touch ./crates/sm64js-common/src/lib.rs && \
    mkdir -p ./crates/sm64js-db/src && \
    touch ./crates/sm64js-db/src/lib.rs && \
    mkdir -p ./crates/sm64js-env/src && \
    touch ./crates/sm64js-env/src/lib.rs && \
    mkdir -p ./crates/sm64js-proto/src && \
    touch ./crates/sm64js-proto/src/lib.rs && \
    mkdir -p ./crates/sm64js-ws/src && \
    touch ./crates/sm64js-ws/src/lib.rs

# compile dependencies
RUN cargo fetch
RUN cargo build --release
RUN rm ./crates/sm64js/src/*.rs && \
    rm ./crates/sm64js-api/src/*.rs && \
    rm ./crates/sm64js-auth/src/*.rs && \
    rm ./crates/sm64js-common/src/*.rs && \
    rm ./crates/sm64js-db/src/*.rs && \
    rm ./crates/sm64js-env/src/*.rs && \
    rm ./crates/sm64js-proto/src/*.rs && \
    rm ./crates/sm64js-ws/src/*.rs

# compile project
COPY ./crates ./crates
COPY ./proto ./proto
# RUN cat ./crates/sm64js-db/src/lib.rs
RUN rm ./target/release/deps/sm64js* && \
    rm -r ./target/release/.fingerprint/sm64js*
RUN cargo build --release --features docker
