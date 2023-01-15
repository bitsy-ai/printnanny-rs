###############################
# Common defaults/definitions #
###############################

comma := ,

# Checks two given strings for equality.
eq = $(if $(or $(1),$(2)),$(and $(findstring $(1),$(2)),\
                                $(findstring $(2),$(1))),1)

OS_NAME := $(shell uname -s)




###########
# Aliases #
###########

check: fmt lint doc

fmt: cargo.fmt

lint: cargo.lint

doc: cargo.doc


##################
# Cargo commands #
##################

# Format Rust sources with rustfmt.
#
# Usage:
#	make cargo.fmt [check=(no|yes)]

cargo.fmt:
	cargo +nightly fmt --all $(if $(call eq,$(check),yes),-- --check,)


# Lint Rust sources with Clippy.
#
# Usage:
#	make cargo.lint

cargo.lint:
	cargo clippy --all -- -D clippy::pedantic -D warnings



# Generate Rust docs.
#
# Usage:
#	make cargo.doc

cargo.doc:
	cargo doc --all-features


##################
# .PHONY section #
##################

.PHONY: fmt lint \
        cargo.fmt cargo.lint