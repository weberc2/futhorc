FROM rustlang/rust:stable

WORKDIR /workspace

COPY . .

RUN cargo build --release && cp ./target/release/futhorc /bin/futhorc
