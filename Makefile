
VERSION ?= 0.2.1
TMP_DIR ?= .tmp
RELEASE_CHANNEL ?= main

$(TMP_DIR)/printnanny_license.zip:
	PRINTNANNY_INSTALL_DIR=$(TMP_DIR) ./tools/download-license.sh

$(TMP_DIR):
	mkdir -p $(TMP_DIR)

$(TMP_DIR)/.venv:
	python3 -m venv $(TMP_DIR)/.venv

ansible: $(TMP_DIR)/.venv
	$(TMP_DIR)/.venv/bin/pip install --upgrade wheel setuptools pip
	$(TMP_DIR)/.venv/bin/pip install ansible
	$(TMP_DIR)/.venv/bin/ansible-galaxy install collection git+https://github.com/bitsy-ai/ansible-collection-printnanny.git,$(RELEASE_CHANNEL)

test-license: $(TMP_DIR)/printnanny_license.zip

clean:
	rm -rf $(TMP_DIR)

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

patch:
	cargo release patch --workspace --execute --tag

minor:
	cargo release minor --workspace --execute --tag

major:
	cargo release major --workspace --execute --tag

test-profile:
	./tools/test-profile.sh