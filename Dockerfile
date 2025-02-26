FROM rust:1.85.0 AS prepare

WORKDIR /usr/src/acup2p

RUN USER=root cargo init --lib
COPY ./rust/Cargo.toml ./Cargo.toml
COPY ./rust/bin ./bin
RUN cargo build --release
RUN rm src/*.rs
RUN rm ./target/release/deps/acup2p*

FROM rust:1.85.0 AS build

WORKDIR /usr/src/acup2p

COPY ./rust .
COPY --from=prepare /usr/src/acup2p/target ./target
RUN cargo build --release
