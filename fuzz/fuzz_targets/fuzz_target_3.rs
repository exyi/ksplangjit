#![no_main]

use std::fmt;

use arbitrary::Arbitrary;
use ksplang::compiler::test_utils::verify_vm_repro;
use ksplang_fuzz::ArbitraryOp;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary)]
struct FuzzInput {
    program: Vec<ArbitraryOp>,
    trace_input: Vec<i64>,
    input: Vec<i64>,
}

impl fmt::Debug for FuzzInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "#[test]")?;
        writeln!(f, "fn fuzz_vm_repro() {{")?;
        write!(f, "    let ops = vec![ ")?;
        for (i, op) in self.program.iter().map(|op| op.0).enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{:?}", op)?;
        }
        writeln!(f, " ];")?;
        writeln!(f, "    verify_vm_repro(ops, vec!{:?}, vec!{:?});", self.trace_input, self.input)?;
        writeln!(f, "}}")
    }
}

fuzz_target!(|data: FuzzInput| {
    let mut data = data;
    if data.program.is_empty() || data.input.is_empty() { return }

    if data.trace_input.is_empty() {
        data.input = data.trace_input.clone();
    }

    let ops = data.program.iter().map(|op| op.0).collect::<Vec<_>>();
    verify_vm_repro(ops, data.trace_input, data.input);
});
