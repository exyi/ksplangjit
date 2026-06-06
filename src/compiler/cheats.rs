use super::prelude::*;
use super::{precompiler::{PrecompileStepResult, Precompiler, TraceProvider}};
use crate::ops::Op;
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

const CHEAT_BITNOT2: [Op; 284] = [
    DigitSum, DigitSum, LenSum, DigitSum, Funkcia, DigitSum, Increment, Increment, Increment, Median, DigitSum, DigitSum, Increment, Gcd2, Increment, Max, DigitSum, DigitSum, Modulo, Qeq,
    DigitSum, DigitSum, DigitSum, Increment, Increment, Qeq, Pop2, DigitSum, DigitSum, TetrationItersNum, DigitSum, Praise, Qeq, Qeq, Pop2, Funkcia, Funkcia, Increment, Modulo, Bitshift,
    DigitSum, DigitSum, Gcd2, DigitSum, Increment, Roll, DigitSum, Universal, DigitSum, DigitSum, Pop2, DigitSum, LenSum, Median, Pop2, Pop2, DigitSum, DigitSum, LenSum, DigitSum, Funkcia, DigitSum,
    Increment, Increment, Increment, Median, DigitSum, DigitSum, Increment, Gcd2, Increment, Max, DigitSum, DigitSum, Modulo, Qeq, DigitSum, DigitSum, DigitSum, Increment, Increment, Qeq, Pop2,
    DigitSum, DigitSum, TetrationItersNum, DigitSum, Praise, Qeq, Qeq, Pop2, Funkcia, Funkcia, Increment, Modulo, Bitshift, DigitSum, DigitSum, Gcd2, DigitSum, Increment, Roll, DigitSum, Universal,
    DigitSum, DigitSum, Pop2, DigitSum, LenSum, Median, Pop2, Pop2, DigitSum, DigitSum, LenSum, Increment, DigitSum, LenSum, Increment, Increment, Increment, Universal, DigitSum, DigitSum, LenSum,
    DigitSum, Funkcia, Increment, Universal, DigitSum, DigitSum, LenSum, DigitSum, Funkcia, Increment, Praise, Qeq, Pop2, Pop2, Funkcia, Increment, Bitshift, Pop2, Pop2, Pop2, Increment,
    DigitSum, DigitSum, LenSum, DigitSum, Funkcia, Increment, DigitSum, DigitSum, Modulo, Qeq, DigitSum, DigitSum, LenSum, DigitSum, Funkcia, Increment, Universal, DigitSum, DigitSum, LenSum, DigitSum,
    Funkcia, Increment, Praise, Qeq, Qeq, Funkcia, And, Pop2, Pop2, DigitSum, Funkcia, DigitSum, DigitSum, LenSum, DigitSum, Funkcia, Increment, DigitSum, Increment, Roll, BranchIfZero, Pop2,
    DigitSum, DigitSum, LenSum, DigitSum, Funkcia, Increment, Praise, Qeq, Qeq, Remainder, Bitshift, Remainder, Pop2, DigitSum, Pop, Jump, Pop2, Pop, DigitSum, DigitSum, LenSum, DigitSum, Funkcia,
    Increment, Praise, Qeq, Pop2, Pop2, Funkcia, Increment, Bitshift, Pop2, Pop2, Pop2, Increment, DigitSum, DigitSum, LenSum, DigitSum, Funkcia, Increment, DigitSum, DigitSum, Modulo, Qeq, Pop2,
    DigitSum, DigitSum, LenSum, Increment, DigitSum, LenSum, DigitSum, Increment, Increment, Bitshift, DigitSum, Increment, Increment, Increment, Pop, Jump, Pop, Pop, DigitSum, DigitSum, LenSum, 
    Increment, DigitSum, DigitSum, DigitSum, Modulo, Qeq, DigitSum, DigitSum, Increment, Roll, DigitSum, DigitSum, LenSum, DigitSum, Funkcia, Increment, DigitSum, DigitSum, Modulo, Qeq, DigitSum,
    DigitSum, LenSum, DigitSum, Funkcia, Universal, DigitSum, Pop
];
const CHEAT_BITNOT1: [Op; 284] = [
    DigitSum, DigitSum, LenSum, DigitSum, Funkcia, DigitSum, Increment, Increment, Increment, Median, DigitSum, DigitSum, Increment, Gcd2, Increment, Max, DigitSum, DigitSum, Modulo, Qeq,
    DigitSum, DigitSum, DigitSum, Increment, Increment, Qeq, Pop2, DigitSum, Jump, Increment, DigitSum, Praise, Qeq, Qeq, Pop2, Funkcia, Funkcia, Increment, Modulo, Bitshift, DigitSum, DigitSum,

    Gcd2, DigitSum, Increment, Roll, DigitSum, Universal, DigitSum, DigitSum, Pop2, DigitSum, LenSum, Median, Pop2, Pop2, DigitSum, DigitSum, LenSum, DigitSum, Funkcia, DigitSum, Increment,
    Increment, Increment, Median, DigitSum, DigitSum, Increment, Gcd2, Increment, Max, DigitSum, DigitSum, Modulo, Qeq, DigitSum, DigitSum, DigitSum, Increment, Increment, Qeq, Pop2, DigitSum, Jump, 
    Increment, DigitSum, Praise, Qeq, Qeq, Pop2, Funkcia, Funkcia, Increment, Modulo, Bitshift, DigitSum, DigitSum, Gcd2, DigitSum, Increment, Roll, DigitSum, Universal, DigitSum, DigitSum, Pop2,

    DigitSum, LenSum, Median, Pop2, Pop2, DigitSum, DigitSum, LenSum, Increment, DigitSum, LenSum, Increment, Increment, Increment, Universal, DigitSum, DigitSum, LenSum, DigitSum, Funkcia,
    Increment, Universal, DigitSum, DigitSum, LenSum, DigitSum, Funkcia, Increment, Praise, Qeq, Pop2, Pop2, Funkcia, Increment, Bitshift, Pop2, Pop2, Pop2, Increment, DigitSum, DigitSum, LenSum,
    DigitSum, Funkcia, Increment, DigitSum, DigitSum, Modulo, Qeq, DigitSum, DigitSum, LenSum, DigitSum, Funkcia, Increment, Universal, DigitSum, DigitSum, LenSum, DigitSum, Funkcia, Increment,
    Praise, Qeq, Qeq, Funkcia, And, Pop2, Pop2, DigitSum, Funkcia, DigitSum, DigitSum, LenSum, DigitSum, Funkcia, Increment, DigitSum, Increment, Roll, BranchIfZero, Pop2, DigitSum, DigitSum, LenSum,
    DigitSum, Funkcia, Increment, Praise, Qeq, Qeq, Remainder, Bitshift, Remainder, Pop2, DigitSum, Pop, Jump, Pop2, Pop, DigitSum, DigitSum, LenSum, DigitSum, Funkcia, Increment, Praise, Qeq,
    Pop2, Pop2, Funkcia, Increment, Bitshift, Pop2, Pop2, Pop2, Increment, DigitSum, DigitSum, LenSum, DigitSum, Funkcia, Increment, DigitSum, DigitSum, Modulo, Qeq, Pop2, DigitSum, DigitSum,
    LenSum, Increment, DigitSum, LenSum, DigitSum, Increment, Increment, Bitshift, DigitSum, Increment, Increment, Increment, Pop, Jump, Pop, Pop, DigitSum, DigitSum, LenSum, Increment, DigitSum,
    DigitSum, DigitSum, Modulo, Qeq, DigitSum, DigitSum, Increment, Roll, DigitSum, DigitSum, LenSum, DigitSum, Funkcia, Increment, DigitSum, DigitSum, Modulo, Qeq, DigitSum, DigitSum, LenSum,
    DigitSum, Funkcia, Universal, DigitSum, Pop
];

fn move_pos<TP: TraceProvider>(p: &mut Precompiler<TP>,
                               skip_len: usize,
                               instrs: usize) {
    p.position += skip_len - 1;
    p.g.current_block_mut().ksplang_instr_count += (instrs - 1) as u32;
    p.step_count += instrs - 1;
}

fn move_instr_cond<TP: TraceProvider>(p: &mut Precompiler<TP>,
                                      cond: Condition<ValueId>,
                                      instr: i64) {
    if instr == 0 || cond == Condition::False { return }

    if instr < 0 {
        p.g.current_block_mut().ksplang_instr_count -= (-instr) as u32;
        return move_instr_cond(p, cond.neg(), -instr);
    }

    let c = p.g.store_constant(instr);
    p.g.push_instr(OptOp::KsplangOpsIncrement(cond), &[c], false, None, None);
}

fn cheat_push_constant<TP: TraceProvider>(p: &mut Precompiler<TP>,
                                          value: i64,
                                          instrs: usize) -> PrecompileStepResult {
    p.g.peek_stack(); // need to peek stack so it behaves equivalently to running the program
    let c = p.g.store_constant(value);
    p.g.stack.push(c);
    move_pos(p, instrs, instrs);
    PrecompileStepResult::Continue
}

fn cheat_dup_simple<TP: TraceProvider>(p: &mut Precompiler<TP>,
                                       skip_len: usize,
                                       instrs: usize) -> PrecompileStepResult {
    let x = p.g.peek_stack();
    p.g.stack.push(x);

    move_pos(p, skip_len, instrs);
    PrecompileStepResult::Continue
}

fn cheat_dup_with_branch<TP: TraceProvider>(p: &mut Precompiler<TP>,
                                            skip_len: usize,
                                            base_instrs: usize,
                                            branching_condition: impl Fn(ValueId) -> Condition<ValueId>,
                                            cond_instrs: i64) -> PrecompileStepResult {
    let x = p.g.peek_stack();
    p.g.stack.push(x);

    move_pos(p, skip_len, base_instrs);

    let cond = branching_condition(x);
    move_instr_cond(p, cond, cond_instrs);

    PrecompileStepResult::Continue
}

fn cheat_bitnot<TP: TraceProvider>(p: &mut Precompiler<TP>,
                                   skip_len: usize,
                                   instr_min: usize,
                                   instr_le3: usize,
                                   instr_gt3: usize) -> PrecompileStepResult {

    let x = p.g.pop_stack();
    let (out, _) = p.g.push_instr(OptOp::BinNot, &[x], true, None, None);
    p.g.stack.push(out);

    move_pos(p, skip_len, instr_le3);
    move_instr_cond(p, Condition::Eq(ValueId::C_IMIN, x), instr_min as i64 - instr_le3 as i64);
    move_instr_cond(p, Condition::Leq(ValueId::C_THREE, x), instr_gt3 as i64 - instr_le3 as i64);

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
            r if r.starts_with(&CHEAT_BITNOT1) => return Some(cheat_bitnot(p, 284, 235, 238, 236)),
            r if r.starts_with(&CHEAT_BITNOT2) => return Some(cheat_bitnot(p, 284, 235, 238, 238)),
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
