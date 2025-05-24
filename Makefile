all: clean fmt build docker

clean:
	cargo clean

fmt:
	cargo fmt
	prettier --write static/*.html static/*.css *.md

build:
	cargo build --release

test:
	cargo test

docker:
	docker build -t matthewblair-net:$$(git rev-parse --short HEAD) .

dockerprune:
	docker system prune -a -f

stop:
	docker stop $$(docker ps --quiet --filter ancestor=matthewblair-net) || true

run: docker
	docker run -p 80:80 -p 443:443 -v /var/cache/acme:/var/cache/acme -d --restart=always matthewblair-net:$$(git rev-parse --short HEAD)

restart: docker stop run

dev:
	cargo run

.PHONY: all clean fmt build test docker dockerprune stop run restart dev
