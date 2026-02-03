# KsplangJIT dev tips

Project is **KsplangJIT** - high-performance trace-assisted JIT runtime for [ksplang](ksplang_en.md), a stack-based esoteric language.
The JIT compiles into an SSA-based CFG and then into bytecode (Osmibytecode / OBC).

## Components

- `src/vm.rs` - Reference interpreter + tracing and JIT invocation
- `src/ops.rs` - Defines the 33 ksplang instruction
- `src/compiler/`
  - `simplifier.rs` - CFG simplifications
  - `cfg.rs` - CFG: `BasicBlock` and `GraphBuilder`
    - note that GraphBuilder::push_instr directly invokes the simplifier
  - `precompiler.rs` - Symbolically interprets ksplang and build the CFG
  - `osmibytecode.rs` - defines osmibytecode opcodes
  - `osmibytecode_backend.rs` - compiles CFG to osmibytecode
  - `osmibytecode_vm.rs` - interpreter
  - `config.rs` - JIT configuration via environment variables (see below)

## Concepts

- CFG is SSA, but we use block parameters instead of phi nodes
- ValueId is used for all values. Small constants are predefined as ValueId::C_ONE, other must be created with cfg.store_constant(100)
- CFG can contain deopts:
  - deopt means that we revert to the state in previous Checkpoint instruction
  - it does not necessarily mean error
- error are (for now) always interpreted as deopt
  - we let the reference interpreter handle failures if they ought to happen
- precompiler has a local symbolic stack of `ValueId`s, which will be a small subset of the execution stack at runtime
  - execution stack may be very large, it's the only memory programs have
  - `Pop` op moves value from execution stack to symbolic stack
  - we can perform optimizations on the symbolic values
  - StackSwap, StackRead instructions read from the execution stack and deopt if it would touch the execution stack (as it invalidates optimizations)

## Testing

`cargo test --workspace`

We have fuzz tests verying that JIT behaves the same as reference interpreter. It's run with `cargo +nightly fuzz run fuzz_target_2`, agent SHOULD NOT start this.

We make unit tests from fuzz crashes, its in fuzz/tests/regression_tests.rs.
First named fuzz_repro, then renamed based on the bug which needed to be fixed.

If you are tasked with fixing fuzz_repro, run it with `cargo test -p ksplang-fuzz fuzz_repro` (possibly with 255 verbosity).
If the fix is complex (there is more ways to fix it and requires some decisions), do not attempt right away, describe the issue and ask for confirmation.

## config

Key env variables:

- `KSPLANGJIT_VERBOSITY=0..255` - log level
- `KSPLANGJIT_ALLOW_UPHOISTING=0` - disables hoisting
- `KSPLANGJIT_ALLOW_OSMIBYTE_BACKEND=0` - forces execution of CFG (slower, but less buggy)
- `KSPLANGJIT_TRACE_LIMIT=N` - Max trace length (0 disables tracing)
- defined in src/compiler/config.rs, but there isn't much more useful things

## Code Conventions

- Simpler is better
- IRange/URange (type aliases for `RangeInclusive<i64/u64>`)
- Do not handle unnecessary edge cases. If unsure if it can happen, add assert! / .unwrap / unreachable!. Fuzzer will either give us test case or prove it can't happen.
- Use debug_assert for more expensive expressions or in super hot code (i.e., osmibytecode vm)
- Never run autoformatter
- Import `FxHashMap` as `HashMap`. Use BTreeMap if performance isn't critical or set is unlikely to be big
- `SmallVec` and `ArrayVec` for small, inline-allocated collections, but don't bother in colder code

## Debugging Tips

- println! and dbg! and search
- Add (debug_)?assert! if it seems something is getting into invalid state
- CFG, Osmibytecode and many other structs implement `Display` for readable IR dumps
- most fuzz_repro can be fixed very simply

## Other

Ask when not sure! I'll revert your changes if you do more changes than I expect.
Don't try to write ksplang programs, no human can do it without additional tooling; you can copy existing fragments or build CFGs directly for testing.
