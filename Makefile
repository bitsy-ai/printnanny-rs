
.PHONY: install-fake-services uninstall-fake-services
VERSION ?= latest
TMPDIR ?= .tmp
DEV_MACHINE ?= dev-rpi4
DEV_USER ?= pi

PRINTNANNY_ADMIN_GROUP ?= printnanny-admin
USER ?= $(shell whoami)

PRINTNANNY_WEBAPP_WORKSPACE ?= $(HOME)/projects/octoprint-nanny-webapp


$(TMPDIR):
	mkdir -p $(TMPDIR)

gstd:
	/usr/local/bin/gstd --enable-http-protocol --http-address=0.0.0.0 --http-port=5000

test:
	cargo test --workspace --all-features  

clean:
	rm -rf $(TMPDIR)

settings:
	git clone git@github.com:bitsy-ai/printnanny-settings.git $(TMPDIR)/settings
	

images:
	docker build \
		-f tools/docker/aarch64-unknown-linux-gnu.Dockerfile \
		-t bitsyai/cross-rs:aarch64-unknown-linux-gnu-22.04 tools
	docker push bitsyai/cross-rs:aarch64-unknown-linux-gnu-22.04
	docker build \
		-f tools/docker/armv7-unknown-linux-gnueabihf.Dockerfile \
		-t bitsyai/cross-rs:armv7-unknown-linux-gnueabihf-22.04 tools
	docker push bitsyai/cross-rs:armv7-unknown-linux-gnueabihf-22.04
	docker build \
		-f tools/docker/x86_64-unknown-linux-gnu.Dockerfile \
		-t bitsyai/cross-rs:x86_64-unknown-linux-gnu-22.04 tools
	docker push bitsyai/cross-rs:x86_64-unknown-linux-gnu-22.04

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
	cross build --bin=nats-edge-worker --target=aarch64-unknown-linux-gnu
	rsync --progress -e "ssh -o StrictHostKeyChecking=no" target/aarch64-unknown-linux-gnu/debug/nats-edge-worker $(DEV_USER)@$(DEV_MACHINE).local:~/nats-edge-worker
	ssh -o StrictHostKeyChecking=no $(DEV_USER)@$(DEV_MACHINE) "sudo systemctl stop printnanny-edge-nats && sudo cp nats-edge-worker /usr/bin/nats-edge-worker && sudo systemctl restart printnanny-edge-nats"

	# rsync --progress -e "ssh -o StrictHostKeyChecking=no" target/aarch64-unknown-linux-gnu/debug/printnanny-cli $(DEV_USER)@$(DEV_MACHINE).local:~/printnanny-cli
	# rsync --progress -e "ssh -o StrictHostKeyChecking=no" target/aarch64-unknown-linux-gnu/debug/printnanny-dash $(DEV_USER)@$(DEV_MACHINE).local:~/printnanny-dash
	# ssh -o StrictHostKeyChecking=no $(DEV_USER)@$(DEV_MACHINE) "sudo systemctl stop printnanny*" || echo "Failed to stop printnanny services"
	# ssh -o StrictHostKeyChecking=no $(DEV_USER)@$(DEV_MACHINE) "sudo cp ~/printnanny-cli /usr/bin/printnanny-cli"
	# ssh -o StrictHostKeyChecking=no $(DEV_USER)@$(DEV_MACHINE) "sudo cp ~/printnanny-dash /usr/bin/printnanny-dash"
	# ssh -o StrictHostKeyChecking=no $(DEV_USER)@$(DEV_MACHINE) "sudo systemctl start printnanny*"

dev-gst-bin:
	cross build --bin=printnanny-gst-pipeline --target=aarch64-unknown-linux-gnu
	rsync --progress -e "ssh -o StrictHostKeyChecking=no" target/aarch64-unknown-linux-gnu/debug/printnanny-gst-pipeline $(DEV_USER)@$(DEV_MACHINE).local:~/printnanny-gst-pipeline


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
	sudo cp tools/systemd/moonraker.service /etc/systemd/system/moonraker.service
	sudo cp tools/systemd/klipper.service /etc/systemd/system/klipper.service

	sudo systemctl daemon-reload
	sudo systemctl enable octoprint.service
	sudo systemctl enable moonraker.service
	sudo systemctl enable klipper.service



uninstall-fake-services:
	sudo rm /etc/systemd/system/mainsail.service
	sudo rm /etc/systemd/system/octoprint.service
	sudo rm /etc/systemd/system/printnanny-vision.service
	sudo rm /etc/systemd/system/syncthing.service
	sudo rm /etc/systemd/system/moonraker.service
	sudo rm /etc/systemd/system/klipper.service
	sudo systemctl daemon-reload
