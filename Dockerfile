FROM rust:1.62.0

WORKDIR /workspace

COPY . .

RUN cargo build --release && cp ./target/release/futhorc /bin/futhorc
