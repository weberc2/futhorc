FROM rustlang/rust:nightly

WORKDIR /workspace

COPY . .

RUN cargo build --release && cp ./target/release/futhorc /bin/futhorc
