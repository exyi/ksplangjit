use crate::compiler::utils::u64neg;

use super::prelude::*;

/// Simplify b assuming a is true
pub fn cond_implies(cfg: &GraphBuilder, assume: &Condition<ValueId>, b: &Condition<ValueId>, _at: InstrId) -> Option<Condition<ValueId>> {
    use Condition::*;
    // very naive implementation for now
    if assume == &Condition::False || b == &Condition::True { return Some(Condition::True) }
    if assume == &Condition::True || b == &Condition::False { return Some(b.clone()) }
    if assume == b {
        return Some(Condition::True);
    }
    let b_neg = b.clone().neg();
    if assume == &b_neg {
        return Some(Condition::False);
    }

    match (assume, b) {
        (Eq(a2, a1),   Eq(b1, b2) | Eq(b2, b1))
            if a1 == b1 && a1.is_computed() => {
            return Some(Eq(*a2, *cmp::min(b1, b2)))
        }
        (Lt(a1, a2) | Gt(a2, a1) | NotDivides(a1, a2),   Neq(b1, b2) | Neq(b1, b2))
            if a1 == b1 && a2 == b2 => {
            return Some(Condition::True)
        }
        (Lt(a1, a2) | Gt(a2, a1) | NotDivides(a1, a2),   Eq(b1, b2) | Eq(b1, b2))
            if a1 == b1 && a2 == b2 => {
            return Some(Condition::False)
        }
        (Eq(a1, a2) | Eq(a2, a1),   Neq(b1, b2) | Lt(b1, b2) | Gt(b1, b2) | NotDivides(b1, b2))
            if a1 == b1 && a2 == b2 => {
            return Some(Condition::False)
        }
        (Eq(a1, a2) | Eq(a2, a1),   Leq(b1, b2) | Geq(b1, b2) | Divides(b1, b2))
            if a1 == b1 && a2 == b2 => {
            return Some(Condition::True)
        }

        (Divides(x1, d1), Divides(x2, d2)) if x1 == x2 && let Some(d1) = cfg.get_constant(*d1) && let Some(d2) = cfg.get_constant(*d2) => {
            if d1.is_multiple_of(&d2) {
                return Some(Condition::True)
            }
        }

        _ => {}
    }

    None
}

// struct TraceTree {
//     trace: HashMap<ValueId, InstrId>,
//     path_constraints: Vec<Condition<ValueId>>,
//     branching: Vec<TraceTree>,
// }


// /// Returns a list of all instruction traces that could have produced the given value, up to max_len instructions long.
// /// Multiple traces are returned if the value is produced by a phi node (block param), i.e. control flow is involved.
// pub fn trace_value_origin(cfg: Builder, val: ValueId, max_len: usize, max_count: usize) -> Vec<Vec<InstrId>> {
//     let val1 = self.values[&val];
//     let instr = self.get_instruction(val1.assigned_at).unwrap();
//     let wavefront: Vec<()
// }

// /// Returns a list of possible value combinations for the given ValueIds, if the set is small enough.
// pub fn please_please_find_its_a_constant(&self, max_size: usize, vals: &[ValueId]) -> Option<Vec<Vec<i64>>> {
//     let val_infos = vals.iter().map(|v| self.values[v]).collect::<Vec<_>>();
    
// }

pub fn interesting_implications(g: &mut GraphBuilder, cond: &Condition<ValueId>, at: InstrId) -> Vec<Condition<ValueId>> {
    use Condition::*;

    let mut r = Vec::new();

    match cond {
        Eq(con, b) | Neq(con, b) | Gt(con, b) | Geq(con, b) | Lt(con, b) | Leq(con, b)
            if let Some(a) = g.get_constant(*con) => {

            if let Some(def) = g.get_defined_at(*b) {
                match def.op {
                    OptOp::Min => {
                        let range = g.val_range_at(*b, at);
                        // C == min(C, x, y)  =>  C <= x  &  C <= y
                        // C <= min(x, y)     =>  C <= x  &  C <= y
                        if matches!(cond, Leq(_, _)) || matches!(cond, Eq(_, _) if *range.end() == a) {
                            return def.inputs.iter().map(|x| Condition::Leq(*con, *x)).collect()
                        }
                        // C < min(x, y)      =>  C < x  &  C < y
                        // C != min(x, y) with C as the lower bound also means C < min(x, y)
                        if matches!(cond, Lt(_, _)) || matches!(cond, Neq(_, _) if *range.start() == a) {
                            return def.inputs.iter().map(|x| Condition::Lt(*con, *x)).collect()
                        }
                    }
                    OptOp::Max => {
                        let range = g.val_range_at(*b, at);
                        // C == max(C, x, y)  =>  C >= x  &  C >= y
                        // C >= max(x, y)     =>  C >= x  &  C >= y
                        if matches!(cond, Geq(_, _)) || matches!(cond, Eq(_, _) if *range.start() == a) {
                            return def.inputs.iter().map(|x| Condition::Geq(*con, *x)).collect()
                        }
                        // C > max(x, y)      =>  C > x  &  C > y
                        // C != max(x, y) with C as the upper bound also means C > max(x, y)
                        if matches!(cond, Gt(_, _)) || matches!(cond, Neq(_, _) if *range.end() == a) {
                            return def.inputs.iter().map(|x| Condition::Gt(*con, *x)).collect()
                        }
                    }

                    _ => {}
                }
            }
        }

        &Divides(x, div) => {
            let x_range = g.val_range_at(x, at);
            if !div.is_constant() {
                r.push(Neq(ValueId::C_ZERO, div));
                let x_max = *abs_range(&x_range).end();
                if !x_range.contains(&0) {
                    r.push(Geq(g.store_constant(x_max.saturating_into()), div));
                    r.push(Leq(g.store_constant(u64neg(x_max)), div));
                }
            }
        }
        _ => {}
    }

    return r;
}


#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BBOrderInfo {
    pub always_before: HashSet<BlockId>,
    pub always_after: HashSet<BlockId>,
}

pub fn postorder(g: &GraphBuilder) -> Vec<BlockId> {
    let mut visited = vec![false; g.blocks.len()];
    let mut result = vec![];

    fn core(g: &GraphBuilder, id: BlockId, visited: &mut Vec<bool>, result: &mut Vec<BlockId>) {
        let b = &g.blocks[id.0 as usize];
        visited[id.0 as usize] = true;
        for (_, next) in &b.outgoing_jumps {
            if !visited[next.0 as usize] {
                core(g, *next, visited, result);
            }
        }
        result.push(id);
    }

    core(g, BlockId(0), &mut visited, &mut result);

    result
}

pub fn reverse_postorder(g: &GraphBuilder) -> Vec<BlockId> {
    let mut o = postorder(g);
    o.reverse();
    o
}

pub fn dataflow<T: PartialEq>(
    g: &GraphBuilder,
    reverse: bool,
    init: impl Fn(&BasicBlock) -> T,
    step: impl Fn(&BasicBlock, &T, &[&T], &[&T]) -> T
) -> HashMap<BlockId, T> {
    let mut order = postorder(g);
    if reverse {
        order.reverse();
    }
    let mut lookup = vec![usize::MAX; g.blocks.len()];
    for (i, id) in order.iter().enumerate() {
        lookup[id.0 as usize] = i;
    }
    let mut state: Vec<T> = order.iter().map(|id| init(&g.blocks[id.0 as usize])).collect();

    let mut iters = 0;

    loop {
        let next_state: Vec<T> = state.iter().zip(order.iter()).map(|(s, bid)| {
            let b = g.block_(*bid);
            let ins: SmallVec<[&T; 4]> =
                b.incoming_jumps.iter().map(|i| &state[lookup[i.block_id().0 as usize]]).collect();
            let outs: SmallVec<[&T; 4]> =
                b.outgoing_jumps.iter().map(|(_i, b)| &state[lookup[b.0 as usize]]).collect();
            step(b, s, &ins, &outs)
        }).collect();

        if next_state == state {
            return order.into_iter().zip(next_state).collect();
        }

        state = next_state;

        assert!(iters < g.blocks.len() + 10, "{iters} > {}", g.blocks.len());
        iters += 1;
    }
}

pub fn get_max_instructions_executed(g: &GraphBuilder) -> u64 {
    let block_count: Vec<i64> = g.blocks.iter().map(|b| {
        let base = b.ksplang_instr_count as i64;
        let inc = b.instructions.values().filter(|i| matches!(i.op, OptOp::KsplangOpsIncrement(_)))
                                         .map(|i| i.inputs.iter()
                                                          .map(|val| *g.val_range_at(*val, i.id).end())
                                                          .sum::<i64>())
                                         .sum::<i64>();
        base + inc
    }).collect();
    let df = dataflow::<i64>(g, /* reverse */ false,
        |b| block_count[b.id.0 as usize],
        // TODO: add some "check" instructions for handling loops
        |b, _, ins, _outs| block_count[b.id.0 as usize] + *ins.iter().copied().max().unwrap_or(&0)
    );

    return cmp::max(0, *df.values().max().unwrap()) as u64;
}
