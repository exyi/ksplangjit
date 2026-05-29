use std::ops::RangeInclusive;

use crate::{compiler::{ops::{OptOp, ValueId}, osmibytecode::Condition, precompiler::{PrecompileStepResult, Precompiler, TraceProvider}, range_ops::IRange}, ops::Op};
use crate::ops::Op::*;
use crate::ops::Op::DigitSum as CS;

const CHEAT_VZORAKOVA_DUP: [Op; 134] = [
    CS, CS, LenSum, Increment, CS, LenSum, Median, CS, CS, LenSum,
    CS, Funkcia, CS, Increment, CS, Qeq, Universal, CS, CS, LenSum,
    CS, Funkcia, Increment, Bitshift, CS, CS, LenSum, Increment, CS, LenSum,
    Median, CS, CS, LenSum, CS, Funkcia, CS, Increment, CS, Qeq,
    Universal, CS, CS, LenSum, CS, Funkcia, Increment, Bitshift, Pop2, CS,
    CS, LenSum, Increment, CS, LenSum, CS, Increment, Increment, Roll, Median,
    CS, CS, LenSum, CS, Funkcia, Increment, CS, CS, Funkcia, Qeq,
    CS, CS, LenSum, CS, Funkcia, Increment, Bitshift, Pop2, CS, CS,
    LenSum, CS, Funkcia, Universal, Increment, Increment, Increment, CS, CS, CS,
    CS, LenSum, CS, Funkcia, CS, Increment, CS, Qeq, Universal, CS,
    Increment, CS, LenSum, CS, Increment, Increment, Roll, CS, Funkcia, Universal,
    CS, CS, LenSum, CS, Funkcia, Increment, CS, Increment, Increment, Roll,
    CS, CS, LenSum, CS, Funkcia, CS, Increment, CS, Qeq, Universal,
    CS, CS, Funkcia, Universal,
];

const CHEAT_ERIKOVA_DUP: [Op; 61] = [
    CS, CS, LenSum, CS, Funkcia, CS, Increment, Increment, Increment, Median,
    CS, CS, Increment, Gcd2, Increment, Max, CS, CS, Remainder, Qeq,
    CS, CS, CS, Increment, Increment, Qeq, Pop2, CS, CS, Increment,
    Gcd2, CS, CS, CS, CS, Bitshift, Bitshift, CS, Bitshift, CS,
    CS, Pop2, Universal, Bitshift, CS, CS, Gcd2, CS, Increment, Roll,
    CS, Universal, CS, CS, Increment, Gcd2, Increment, Increment, Median, Pop2,
    Pop2,
];

const CHEAT_SEJSELOVA_DUP: [Op; 56] = [
    CS, CS, LenSum, CS, Funkcia, CS, Increment, Increment, Increment, Median,
    CS, CS, Increment, Gcd2, Increment, Max, CS, CS, Modulo, Qeq,
    CS, CS, CS, Increment, Increment, Qeq, Pop2, CS, Jump, Increment,
    CS, Praise, Qeq, Qeq, Pop2, Funkcia, Funkcia, Increment, Modulo, Bitshift,
    CS, CS, Gcd2, CS, Increment, Roll, CS, Universal, CS, CS,
    Pop2, CS, LenSum, Median, Pop2, Pop2,
];

const CHEAT_SEJSELOVA2_DUP: [Op; 56] = [
    CS, CS, LenSum, CS, Funkcia, CS, Increment, Increment, Increment, Median,
    CS, CS, Increment, Gcd2, Increment, Max, CS, CS, Modulo, Qeq,
    CS, CS, CS, Increment, Increment, Qeq, Pop2, CS, CS, TetrationItersNum,
    CS, Praise, Qeq, Qeq, Pop2, Funkcia, Funkcia, Increment, Modulo, Bitshift,
    CS, CS, Gcd2, CS, Increment, Roll, CS, Universal, CS, CS,
    Pop2, CS, LenSum, Median, Pop2, Pop2,
];

const CHEAT_DUP1_TROCHU_JINA: [Op; 137] = [
    CS, CS, LenSum, CS, Funkcia, Increment, Increment, Median, CS, CS,
    LenSum, CS, Funkcia, CS, Increment, CS, Qeq, Universal, CS, CS,
    LenSum, CS, Funkcia, Increment, Bitshift, CS, CS, LenSum, CS, Funkcia,
    Increment, Increment, Median, CS, CS, LenSum, CS, Funkcia, CS, Increment,
    CS, Qeq, Universal, CS, CS, LenSum, CS, Funkcia, Increment, Bitshift,
    Pop2, CS, CS, LenSum, CS, Funkcia, Increment, Increment, CS, Increment,
    Increment, Roll, Median, CS, CS, LenSum, CS, Funkcia, Increment, CS,
    CS, Funkcia, Qeq, CS, CS, LenSum, CS, Funkcia, Increment, Bitshift,
    Pop2, CS, CS, LenSum, CS, Funkcia, Universal, Increment, Increment, Increment,
    CS, CS, CS, CS, LenSum, CS, Funkcia, CS, Increment, CS,
    Qeq, Universal, CS, Increment, CS, LenSum, CS, Increment, Increment, Roll,
    CS, Funkcia, Universal, CS, CS, LenSum, CS, Funkcia, Increment, CS,
    Increment, Increment, Roll, CS, CS, LenSum, CS, Funkcia, CS, Increment,
    CS, Qeq, Universal, CS, CS, Funkcia, Universal,
];

fn cheat_push_constant<TP: TraceProvider>(p: &mut Precompiler<TP>,
                                          value: i64,
                                          instrs: usize) -> PrecompileStepResult {
    let c = p.g.store_constant(value);
    p.g.stack.push(c);
    p.position += instrs - 1;
    p.g.current_block_mut().ksplang_instr_count += (instrs - 1) as u32;
    p.step_count += instrs - 1;
    PrecompileStepResult::Continue
}

fn cheat_dup_simple<TP: TraceProvider>(p: &mut Precompiler<TP>,
                                       skip_len: usize,
                                       instrs: usize) -> PrecompileStepResult {
    let x = p.g.peek_stack();
    p.g.stack.push(x);

    p.position += skip_len - 1;
    p.g.current_block_mut().ksplang_instr_count += (instrs - 1) as u32;
    p.step_count += instrs - 1;
    PrecompileStepResult::Continue
}

fn cheat_dup_with_branch<TP: TraceProvider>(p: &mut Precompiler<TP>,
                                            skip_len: usize,
                                            base_instrs: usize,
                                            branching_condition: impl Fn(ValueId) -> Condition<ValueId>,
                                            cond_instrs: usize) -> PrecompileStepResult {
    let x = p.g.peek_stack();
    p.g.stack.push(x);

    p.position += skip_len - 1;
    p.g.current_block_mut().ksplang_instr_count += (base_instrs - 1) as u32;
    p.step_count += base_instrs - 1;

    let cond = branching_condition(x);
    let increment = p.g.store_constant(cond_instrs as i64);
    p.g.push_instr(OptOp::KsplangOpsIncrement(cond), &[increment], false, None, None);

    PrecompileStepResult::Continue
}

pub fn try_cheat<TP: TraceProvider>(p: &mut Precompiler<TP>) -> Option<PrecompileStepResult> {
    if p.conf.cheat_mode == 0 {
        return None;
    }

    let remaining = &p.ops[p.position..];

    let is_range = |r: IRange| {
        let Some(top_val) = p.g.stack.peek() else { return false };
        let range = p.g.val_range(top_val);
        return range.start() >= r.start() && range.end() <= r.end();
    };
    // let is_constant = |c: i64| {
    //     let Some(top_val) = p.g.stack.peek() else { return false; };
    //     p.g.get_constant(top_val) == Some(c)
    // };

    if p.conf.cheat_mode >= 2 {
        match remaining {
            r if r.starts_with(&CHEAT_DUP1_TROCHU_JINA) => return Some(cheat_dup_simple(p, 137, 137)),
            r if r.starts_with(&CHEAT_VZORAKOVA_DUP) => return Some(cheat_dup_simple(p, 134, 134)),
            r if r.starts_with(&CHEAT_ERIKOVA_DUP) => return Some(cheat_dup_simple(p, 61, 61)),
            r if r.starts_with(&CHEAT_SEJSELOVA_DUP) =>
                return Some(cheat_dup_with_branch(p, 56, 55, |v| Condition::Gt(ValueId::C_THREE, v), 1)),
            r if r.starts_with(&CHEAT_SEJSELOVA2_DUP) => return Some(cheat_dup_simple(p, 56, 56)),
            _ => {}
        }
    }

    match remaining {
        // i64::MAX CS CS lensum CS funkcia ++ praise qeq pop2 pop2 funkcia ++ bitshift pop2 pop2 pop2 ++ CS CS lensum CS funkcia ++ CS CS % qeq
        [CS, CS, LenSum, CS, Funkcia, Increment, Praise, Qeq, Pop2, Pop2, Funkcia, Increment, Bitshift, Pop2, Pop2, Pop2, Increment, CS, CS, LenSum, CS, Funkcia, Increment, CS, CS, Modulo, Qeq, ..] =>
            return Some(cheat_push_constant(p, i64::MAX, 27)),
        // i64::MIN CS CS lensum CS funkcia ++ praise qeq pop2 pop2 funkcia ++ bitshift pop2 pop2 pop2
        [CS, CS, LenSum, CS, Funkcia, Increment, Praise, Qeq, Pop2, Pop2, Funkcia, Increment, Bitshift, Pop2, Pop2, Pop2, ..] =>
            return Some(cheat_push_constant(p, i64::MIN, 16)),
        // -1: CS CS lensum ++ CS CS CS % qeq
        [CS, CS, LenSum, Increment, CS, CS, CS, Modulo | Remainder, Qeq] =>
            return Some(cheat_push_constant(p, -1, 9)),
        // 0: CS CS lensum CS lensum
        [CS, CS, LenSum, CS, Funkcia, ..] =>
            return Some(cheat_push_constant(p, 0, 5)),
        // 0: CS CS lensum ++ CS % (shortest string)
        [CS, CS, LenSum, Increment, CS, Remainder | Modulo, ..] =>
            return Some(cheat_push_constant(p, 0, 6)),
        // 2: CS CS lensum ++ CS lensum (+ variations)
        [CS, CS, LenSum, Increment, CS, LenSum, ..] |
        [CS, CS, Increment, LenSum, CS, LenSum, ..] =>
            return Some(cheat_push_constant(p, 2, 6)),

        // special pattern used in duplication, produces i64::MIN if argument is 0/-1
        // CS CS ^^ CS praise qeq qeq pop2 funkcia funkcia ++ % bitshift  [CS CS gcd CS ++ lroll]
        [CS, CS, TetrationItersNum, CS, Praise, Qeq, Qeq, Pop2, Funkcia, Funkcia, Increment, Modulo, Bitshift, CS, CS, Gcd2, CS, Increment, Roll, ..] if is_range(-1..=0) => {
            cheat_push_constant(p, i64::MIN, 19);
            let a = p.g.pop_stack();
            let b = p.g.pop_stack();
            p.g.stack.push(a);
            p.g.stack.push(b);
            return Some(PrecompileStepResult::Continue);
        }
        [CS, CS, TetrationItersNum, CS, Praise, Qeq, Qeq, Pop2, Funkcia, Funkcia, Increment, Modulo, Bitshift, ..] if is_range(-1..=0) => {
            return Some(cheat_push_constant(p, i64::MIN, 13))
        }

        _ => {}
    }

    None
}


#[test]
fn lala() {
    use crate::{compiler::tests as t, parser::parse_program};
    assert_eq!(&CHEAT_SEJSELOVA_DUP, &parse_program(t::SEJSELOVA_DUP).unwrap()[..]);
    assert_eq!(&CHEAT_SEJSELOVA2_DUP, &parse_program(t::SEJSELOVA2_DUP).unwrap()[..]);
    assert_eq!(&CHEAT_ERIKOVA_DUP, &parse_program(t::ERIKOVA_DUP).unwrap()[..]);
    assert_eq!(&CHEAT_VZORAKOVA_DUP, &parse_program(t::VZORAKOVA_DUP).unwrap()[..]);
}
