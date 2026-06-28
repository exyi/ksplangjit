set positional-arguments
set shell := ["bash", "-c"]

@test *ARGS:
	KSPLANGJIT_TRIGGER_COUNT=3 KSPLANGJIT_CHEAT=0 cargo test --workspace --no-fail-fast {{ARGS}}

@test-combi *ARGS:
	set -x
	KSPLANGJIT_TRIGGER_COUNT=3 KSPLANGJIT_CHEAT=0 cargo test --release --workspace {{ARGS}}
	KSPLANGJIT_TRIGGER_COUNT=3 KSPLANGJIT_CHEAT=1 cargo test --release --workspace {{ARGS}}
	KSPLANGJIT_TRIGGER_COUNT=3 KSPLANGJIT_CHEAT=2 cargo test --release --workspace {{ARGS}}
	KSPLANGJIT_TRIGGER_COUNT=3 KSPLANGJIT_CHEAT=0 KSPLANGJIT_YIELD_INTERVAL=30 cargo test --release --workspace {{ARGS}}
	KSPLANGJIT_TRIGGER_COUNT=3 KSPLANGJIT_ALLOW_OSMIBYTE_BACKEND=0 KSPLANGJIT_CHEAT=0 KSPLANGJIT_YIELD_INTERVAL=30 cargo test --release --workspace {{ARGS}}
	# TODO: 
	# KSPLANGJIT_TRIGGER_COUNT=3 KSPLANGJIT_ERROR_AS_DEOPT=0 KSPLANGJIT_CHEAT=0 KSPLANGJIT_YIELD_INTERVAL=30 cargo test --workspace {{ARGS}}

@ftest-check *ARGS:
	#!/usr/bin/env bash
	cargo +nightly fuzz build --codegen-units 16 --sanitizer none
	if ! find fuzz/artifacts/fuzz_target_2 -type f | rg 'crash-' | KSPLANGJIT_VERBOSITY=0 parallel -X -j24 --joblog run.log --bar -n 1 "target/x86_64-unknown-linux-gnu/release/fuzz_target_2 {} -runs=0 &> /dev/null"; then
		awk -F'\t' '$7 != 0 {print "FAILED (exit " $7 "): " $9}' run.log
		exit 1
	else
		echo "OK"
	fi
	# find fuzz/artifacts/fuzz_target_1 -type f | rg 'crash-' | KSPLANGJIT_VERBOSITY=0 parallel -X -j24 --joblog run.log --bar -n 1 "target/x86_64-unknown-linux-gnu/release/fuzz_target_1 {} -runs=0 &> /dev/null"

@ftest-corpus *ARGS:
	cargo +nightly fuzz build --codegen-units 16 --sanitizer none
	find fuzz/corpus/fuzz_target_2/ -type f | KSPLANGJIT_VERBOSITY=0 parallel -X -j24 --joblog run.log --bar -n 64 "target/x86_64-unknown-linux-gnu/release/fuzz_target_2 {} -runs=0 &> /dev/null"
	awk -F'\t' '$7 != 0 {print "FAILED (exit " $7 "): " $9}' run.log

