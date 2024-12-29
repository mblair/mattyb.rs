# This is largely borrowed from lychee's Dockerfile, which does a good job of caching dependencies.
FROM rust:bookworm AS builder

RUN USER=root cargo new --bin mattyb
WORKDIR /mattyb

COPY /Cargo.toml Cargo.toml
RUN cargo build --release && \
	rm src/*.rs

COPY . ./
RUN rm ./target/release/deps/mattyb* && \
	cargo build --release

FROM debian:bookworm-slim

RUN apt update && \
	DEBIAN_FRONTEND=noninteractive apt install -y \
		--no-install-recommends ca-certificates tzdata && \
	rm -rf /var/cache/debconf/* && \
	apt clean && \
    apt autoremove -y && \
	rm -rf /var/lib/apt/lists/*

COPY --from=builder /mattyb/target/release/mattyb /usr/local/bin/mattyb
ENTRYPOINT [ "mattyb" ]
CMD [ "-d", "matthewblair.net", "-c", "/var/cache/acme", "--prod" ]
