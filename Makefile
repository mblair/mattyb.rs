all: clean fmt build docker

clean:
	cargo clean

fmt:
	cargo fmt
	shfmt -w *.sh

update:
	cargo update

distupdate:
	cargo upgrade --incompatible

build:
	cargo build

docker:
	# TODO: fix this to work when there's no commit metadata
	#docker build -t web:$$(git rev-parse --short HEAD) .
	docker build -t web:latest .

dockerprune:
	docker system prune -a -f

stop:
	docker stop $$(docker ps --quiet)

run: docker
	#docker run -p 80:80 -p 443:443 -v /var/cache/acme:/var/cache/acme -d --restart=always web:$$(git rev-parse --short HEAD) -- -c /var/cache/acme
	docker run -p 80:80 -p 443:443 -v /var/cache/acme:/var/cache/acme -d --restart=always web:latest

restart: docker stop run
