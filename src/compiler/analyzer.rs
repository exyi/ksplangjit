use super::prelude::*;

/// Simplify b assuming a is true
pub fn cond_implies(_cfg: &GraphBuilder, assume: &Condition<ValueId>, b: &Condition<ValueId>, _at: InstrId) -> Option<Condition<ValueId>> {
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
