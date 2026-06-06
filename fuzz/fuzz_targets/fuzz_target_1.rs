#![no_main]

use std::fmt;

use ksplang_fuzz::ArbitraryOp;
use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;
use ksplang::{compiler::test_utils::ReproData};

#[derive(Arbitrary)]
struct FuzzInput {
    program: Vec<ArbitraryOp>,
    input: Vec<i64>,
}

impl std::fmt::Debug for FuzzInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Display::fmt(&to_repro(&self), f)
    }
}

fn to_repro(i: &FuzzInput) -> ReproData {
    ReproData::new(i.program.iter().map(|op| op.0).collect::<Vec<_>>(), i.input.clone())
}

fuzz_target!(|data: FuzzInput| {
    let r = to_repro(&data);

    r.verify();
});

