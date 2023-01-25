FROM ekidd/rust-musl-builder AS builder

RUN sudo apt update -y && sudo apt install -y sqlite3

ADD --chown=rust:rust . ./
ADD shorturl /home/rust/src

RUN cargo build --release

# FROM aa9a3e522602 AS builder

#######################

FROM scratch

COPY --from=builder /home/rust/src/target/x86_64-unknown-linux-musl/release/short-url /
CMD ["/short-url"]
