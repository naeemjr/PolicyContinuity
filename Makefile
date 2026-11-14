CARGO ?= cargo
# Dependencies are fetched once (`cargo fetch --locked`); every target below
# then runs without network access.
export CARGO_NET_OFFLINE ?= true

FINAL := results/final
DEV := results/dev
VERIFY := target/verify-final
# Temporary workspaces are created here, inside the project, never in the
# system temporary directory.
TMP := target/pc-tmp

# environment.json records the kernel the run used. It is the one output
# expected to differ between machines, so it is not checksummed.
CHECKSUM = find . -type f ! -name SHA256SUMS ! -name environment.json | sort | xargs sha256sum > SHA256SUMS

.PHONY: all build fmt fmt-check lint unit-test integration-test test check \
        dev final verify quick clean

all: check

build:
	$(CARGO) build --locked --release

fmt:
	$(CARGO) fmt

fmt-check:
	$(CARGO) fmt --check

lint:
	$(CARGO) clippy --locked --all-targets -- -D warnings

unit-test:
	$(CARGO) test --locked --lib

integration-test:
	$(CARGO) test --locked --test '*'

test: unit-test integration-test

# Everything CI runs, except the reproducibility rerun.
check: fmt-check lint build test

# Quick look at one development seed. Not a protocol run.
quick:
	$(CARGO) run --locked --release --quiet -- run --seed 1 --temp-parent $(TMP)

# Development run: the 20 development seeds only. No verdict is computed.
dev:
	$(CARGO) run --locked --release --quiet -- dev --out $(DEV) --temp-parent $(TMP)
	cd $(DEV) && $(CHECKSUM)

# The frozen held-out evaluation: every table and figure under
# results/final, plus checksums.
final:
	$(CARGO) run --locked --release --quiet -- final --out $(FINAL) --temp-parent $(TMP)
	cd $(FINAL) && $(CHECKSUM)

# Rerun the final evaluation into a clean directory and compare it, byte for
# byte, with the committed results.
verify:
	rm -rf $(VERIFY)
	$(CARGO) run --locked --release --quiet -- final --out $(VERIFY) --temp-parent $(TMP) > /dev/null
	cd $(VERIFY) && sha256sum --check --quiet ../../$(FINAL)/SHA256SUMS
	@echo "verified: a clean rerun reproduces $(FINAL) exactly"

clean:
	$(CARGO) clean
