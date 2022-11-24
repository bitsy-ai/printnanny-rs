
.PHONY: install-fake-services uninstall-fake-services
VERSION ?= latest
TMPDIR ?= .tmp
DEV_MACHINE ?= pn-debug
DEV_USER ?= root

PRINTNANNY_ADMIN_GROUP ?= printnanny-admin
USER ?= $(shell whoami)

PRINTNANNY_WEBAPP_WORKSPACE ?= $(HOME)/projects/octoprint-nanny-webapp

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

$(TMPDIR)/printnanny-$(DEV_MACHINE).zip: $(TMPDIR)
	make -C $(PRINTNANNY_WEBAPP_WORKSPACE) $(TMPDIR)/printnanny-$(DEV_MACHINE).zip DEV_CONFIG=$(TMPDIR)/printnanny-$(DEV_MACHINE).zip
	cp $(PRINTNANNY_WEBAPP_WORKSPACE)/$(TMPDIR)/printnanny-$(DEV_MACHINE).zip $(TMPDIR)/printnanny-$(DEV_MACHINE).zip

devconfig: $(TMPDIR)/printnanny-$(DEV_MACHINE).zip
	PRINTNANNY_SETTINGS=$(PWD)/env/Local.toml cargo run --bin printnanny-cli -- -v config init


dev-build:
	cross build --workspace --target=aarch64-unknown-linux-gnu
	rsync --progress -e "ssh -o StrictHostKeyChecking=no" target/aarch64-unknown-linux-gnu/debug/cam $(DEV_USER)@$(DEV_MACHINE).local:~/printnanny-cam

	# rsync --progress -e "ssh -o StrictHostKeyChecking=no" target/aarch64-unknown-linux-gnu/debug/printnanny-cli $(DEV_USER)@$(DEV_MACHINE).local:~/printnanny-cli
	# rsync --progress -e "ssh -o StrictHostKeyChecking=no" target/aarch64-unknown-linux-gnu/debug/printnanny-dash $(DEV_USER)@$(DEV_MACHINE).local:~/printnanny-dash
	# ssh -o StrictHostKeyChecking=no $(DEV_USER)@$(DEV_MACHINE) "sudo systemctl stop printnanny*" || echo "Failed to stop printnanny services"
	# ssh -o StrictHostKeyChecking=no $(DEV_USER)@$(DEV_MACHINE) "sudo cp ~/printnanny-cli /usr/bin/printnanny-cli"
	# ssh -o StrictHostKeyChecking=no $(DEV_USER)@$(DEV_MACHINE) "sudo cp ~/printnanny-dash /usr/bin/printnanny-dash"
	# ssh -o StrictHostKeyChecking=no $(DEV_USER)@$(DEV_MACHINE) "sudo systemctl start printnanny*"

gst-image:
	docker build \
		-f gst/Dockerfile \
		-t bitsyai/printnanny-gst \
		.

lint:
	cargo clippy --workspace

dev:
	docker-compose -f docker/local.yml up

install-group:
	sudo groupadd $(PRINTNANNY_ADMIN_GROUP) || echo $1
	sudo usermod -a -G $(PRINTNANNY_ADMIN_GROUP) $(USER)

install-polkit-rules:
	sudo mkdir -p /etc/polkit-1/rules.d/
	sudo install -m 0644 tools/polkit/printnanny.rules /etc/polkit-1/rules.d/printnanny.rules

# Debian-derived distros are still stuck on Polkit 105, which requires the older pkla syntax
install-polkit-pkla:
	sudo install -m 0644 tools/polkit/printnanny.pkla /etc/polkit-1/localauthority/50-local.d/printnanny.pkla


install-fake-services:
	sudo cp tools/systemd/mainsail.service /etc/systemd/system/mainsail.service
	sudo cp tools/systemd/octoprint.service /etc/systemd/system/octoprint.service
	sudo cp tools/systemd/printnanny-vision.service /etc/systemd/system/printnanny-vision.service
	sudo cp tools/systemd/syncthing.service /etc/systemd/system/syncthing.service
	sudo systemctl daemon-reload
	sudo systemctl enable octoprint.service

uninstall-fake-services:
	sudo rm /etc/systemd/system/mainsail.service
	sudo rm /etc/systemd/system/octoprint.service
	sudo rm /etc/systemd/system/printnanny-vision.service
	sudo rm /etc/systemd/system/syncthing.service
	sudo systemctl daemon-reload
