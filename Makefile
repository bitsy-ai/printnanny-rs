
VERSION ?= latest
TMPDIR ?= .tmp
DEV_MACHINE ?= pn-dev
DEV_USER ?= root

PRINTNANNY_WEBAPP_WORKSPACE ?= $(HOME)/projects/octoprint-nanny-webapp

$(TMPDIR)/printnanny_license.zip:
	PRINTNANNY_INSTALL_DIR=$(TMPDIR) ./tools/download-license.sh

$(TMPDIR):
	mkdir -p $(TMPDIR)

test-license: $(TMPDIR)/printnanny_license.zip

clean:
	rm -rf $(TMPDIR)

images:
	docker build \
		-f tools/docker/aarch64-unknown-linux-gnu.Dockerfile \
		-t bitsyai/cross:aarch64-unknown-linux-gnu tools/docker
	docker push bitsyai/cross:aarch64-unknown-linux-gnu
	docker build \
		-f tools/docker/armv7-unknown-linux-gnueabihf.Dockerfile \
		-t bitsyai/cross:armv7-unknown-linux-gnueabihf tools/docker
	docker push bitsyai/cross:armv7-unknown-linux-gnueabihf
	docker build \
		-f tools/docker/x86_64-unknown-linux-gnu.Dockerfile \
		-t bitsyai/cross:x86_64-unknown-linux-gnu tools/docker
	docker push bitsyai/cross:x86_64-unknown-linux-gnu

patch:
	cargo release patch --workspace --execute --tag

minor:
	cargo release minor --workspace --execute --tag

major:
	cargo release major --workspace --execute --tag

$(TMPDIR)/PrintNanny-$(DEV_MACHINE).zip: $(TMPDIR)
	make -C $(PRINTNANNY_WEBAPP_WORKSPACE) $(TMPDIR)/PrintNanny-$(DEV_MACHINE).zip DEV_CONFIG=$(TMPDIR)/PrintNanny-$(DEV_MACHINE).zip
	cp $(PRINTNANNY_WEBAPP_WORKSPACE)/$(TMPDIR)/PrintNanny-$(DEV_MACHINE).zip $(TMPDIR)/PrintNanny-$(DEV_MACHINE).zip

devconfig:$(TMPDIR)/PrintNanny-$(DEV_MACHINE).zip
	PRINTNANNY_CONFIG=$(PWD)/env/Local.toml cargo run --bin printnanny-cli -- -v config init


dev-build:
	cross build --workspace --target=aarch64-unknown-linux-gnu
	rsync --progress -e "ssh -o StrictHostKeyChecking=no" target/aarch64-unknown-linux-gnu/debug/printnanny-cli $(DEV_USER)@$(DEV_MACHINE):~/printnanny-cli
	rsync --progress -e "ssh -o StrictHostKeyChecking=no" target/aarch64-unknown-linux-gnu/debug/printnanny-dash $(DEV_USER)@$(DEV_MACHINE):~/printnanny-dash
	ssh -o StrictHostKeyChecking=no $(DEV_USER)@$(DEV_MACHINE) "sudo systemctl stop printnanny*" || echo "Failed to stop printnanny services"
	ssh -o StrictHostKeyChecking=no $(DEV_USER)@$(DEV_MACHINE) "sudo cp ~/printnanny-cli /usr/bin/printnanny-cli"
	ssh -o StrictHostKeyChecking=no $(DEV_USER)@$(DEV_MACHINE) "sudo cp ~/printnanny-dash /usr/bin/printnanny-dash"
	ssh -o StrictHostKeyChecking=no $(DEV_USER)@$(DEV_MACHINE) "sudo systemctl start printnanny*"

gst-image:
	docker build \
		-f gst/Dockerfile \
		-t bitsyai/printnanny-gst \
		.

lint:
	cargo clippy --workspace