
VERSION ?= 0.2.1
images:
	docker build \
		-f docker/aarch64-unknown-linux-gnu.Dockerfile \
		-t bitsyai/cross:aarch64-unknown-linux-gnu-$(VERSION) \
		docker
	docker push bitsyai/cross:aarch64-unknown-linux-gnu-$(VERSION)
	docker build \
		-f docker/armv7-unknown-linux-gnueabihf.Dockerfile \
		-t bitsyai/cross:armv7-unknown-linux-gnueabihf-$(VERSION) \
		docker
	docker push bitsyai/cross:armv7-unknown-linux-gnueabihf-$(VERSION)
	docker build \
		-f docker/x86_64-unknown-linux-gnu.Dockerfile \
		-t bitsyai/cross:x86_64-unknown-linux-gnu-$(VERSION) \
		docker
	docker push bitsyai/cross:x86_64-unknown-linux-gnu-$(VERSION)

run-local:
	mkdir -p $(HOME)/.printnanny/data/
	mkdir -p $(HOME)/.printnanny/settings/
	PRINTNANNY_GCP_PROJECT=print-nanny-sandbox \
	PRINTNANNY_API_CONFIG__BASE_PATH=http://localhost:8000 \
	cargo run -- -vv $(ARGS)

ansible-facts:
	mkdir -p $(PWD)/.tmp/data/
	mkdir -p $(PWD)/.tmp/settings/
	PRINTNANNY_GCP_PROJECT=print-nanny-sandbox \
	PRINTNANNY_API_CONFIG__BASE_PATH=http://localhost:8000 \
	cargo run -q -- ansible-facts