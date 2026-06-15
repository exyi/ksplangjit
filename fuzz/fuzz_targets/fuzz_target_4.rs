#![no_main]

use std::fmt;

use arbitrary::Arbitrary;
use ksplang::{
    compiler::precompiler::TraceProvider,
    ops::Op,
    vm::{self, ActualTracer, RunError, VMOptions},
};
use ksplang_fuzz::ArbitraryOp;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary)]
struct FuzzInput {
    program: Vec<ArbitraryOp>,
    input: Vec<i64>,
    steps: u16,
}

impl fmt::Debug for FuzzInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "#[test]")?;
        writeln!(f, "fn fuzz_tracer_repro() {{")?;
        write!(f, "    let ops = vec![ ")?;
        for (i, op) in self.program.iter().map(|op| op.0).enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{:?}", op)?;
        }
        writeln!(f, " ];")?;
        writeln!(f, "    let input = vec!{:?};", self.input)?;
        writeln!(f, "    let steps = {:?};", steps(self.steps))?;
        writeln!(f, "    // TODO.")?;
        writeln!(f, "}}")
    }
}

fn steps(raw: u16) -> u64 {
    raw as u64 % 512 + 1
}

fn options<'a>(input: &'a [i64], max_stack: usize, stop_after: u64) -> VMOptions<'a> {
    VMOptions::new(input, max_stack, &[], u64::MAX, stop_after)
}

fn ops(input: &FuzzInput) -> Vec<Op> {
    input.program.iter().map(|op| op.0).collect()
}

fn clamp_steps<T>(result: &Result<vm::RunResult<T>, RunError>) -> Option<u64>
where
    T: vm::Tracer,
{
    match result {
        Ok(result) => Some(result.instruction_counter),
        Err(RunError::InstructionFailed { instruction_counter, .. }) => Some(*instruction_counter),
        Err(RunError::RunTooLong { instruction_counter }) => Some(*instruction_counter),
        Err(RunError::StackOverflow | RunError::Timeout | RunError::TracerInterrupt(..)) => None,
    }
}

fuzz_target!(|data: FuzzInput| {
    if data.program.is_empty() { return }

    let ops = ops(&data);
    let n = steps(data.steps);
    let max_stack = data.input.len().saturating_add(1024).max(2048);

    let tracer = ActualTracer::new(&data.input, false, n as u32);
    let traced = vm::run_with_stats(&ops, options(&data.input, max_stack, n), tracer);
    // TODO: deslopify
    let Some(n) = clamp_steps(&traced) else {
        return;
    };
    if n == 0 {
        return;
    }

    let mut traced = match traced {
        Ok(traced) if traced.instruction_counter == n => traced,
        _ => {
            let tracer = ActualTracer::new(&data.input, false, n as u32);
            let Ok(traced) = vm::run_with_stats(&ops, options(&data.input, max_stack, n), tracer) else {
                return;
            };
            traced
        }
    };
    if traced.instruction_counter != n || traced.tracer.ips.len() != n as usize {
        return;
    }

    let Ok(before) = vm::run(&ops, options(&data.input, max_stack, n - 1)) else {
        return;
    };
    let Ok(after) = vm::run(&ops, options(&data.input, max_stack, n)) else {
        return;
    };
    if before.instruction_counter != n - 1 || after.instruction_counter != n {
        return;
    }
    let candidates = traced.tracer.get_results(before.instruction_pointer).collect::<Vec<_>>();
    assert!(!candidates.is_empty());

    let found = candidates.into_iter().any(|(pops, pushes)| {
        let pops = pops as usize;
        if pops > before.stack.len() {
            return false;
        }

        let expected_len = before.stack.len() - pops + pushes.len();
        after.stack.len() == expected_len &&
            after.stack.ends_with(&pushes)
    });

    assert!(found);
});
