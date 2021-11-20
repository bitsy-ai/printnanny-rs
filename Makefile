
VERSION ?= 0.2.1
TMP_DIR ?= .tmp

$(TMP_DIR):
	mkdir -p $(TMP_DIR)

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

run-local:
	mkdir -p 
	PRINTNANNY_GCP_PROJECT=print-nanny-sandbox \
	PRINTNANNY_API_CONFIG__BASE_PATH=http://localhost:8000 \
	cargo run -- -vv -c $(TMP_DIR) $(ARGS)


$(TMP_DIR)/printnanny_license.zip:
	PRINTNANNY_INSTALL_DIR=$(TMP_DIR) ./tools/download-license.sh