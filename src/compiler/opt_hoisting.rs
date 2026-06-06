
use super::prelude::*;
use super::{ops::BeforeOrAfter, utils::{Annotations, RemoveAll, all_equal}};

/// Hoists common instructions from following blocks of the specified predecessor block.
/// Returns true if any hoisting was performed.
pub fn hoist_up(g: &mut GraphBuilder, predecessor: BlockId) -> bool {
    let pred_block = g.block_(predecessor);

    let mut successors = pred_block.following_blocks();
    successors.sort();
    successors.dedup();
    let successors = successors;

    if successors.len() < 2 {
        return false; // TODO: merge?
    }

    for &succ_id in &successors {
        let succ_block = g.block_(succ_id);
        if g.conf.should_log(10) {
            println!("  Attempting hoisting from {succ_block}");
        }
        //   can't safely           would not be productive
        if !succ_block.is_sealed || succ_block.incoming_jumps.len() > 1 {
            return false;
        }
    }

    if g.conf.should_log(10) {
        println!("Running hoisting for {predecessor}: {successors:?}");
        println!("  Attempting hoisting into {pred_block}");
    }

    let mut hoisted_any = false;
    'main: loop {
        let mut candidate_instr = get_common_instructions(g, &successors);

        if candidate_instr.is_empty() {
            break;
        }

        candidate_instr.sort_by_cached_key(|(_, _, ids)| *ids.iter().max().unwrap());

        'candidate: for (op, inputs, instr_ids) in candidate_instr.iter() {
            assert_eq!(instr_ids.len(), successors.len(), "common instruction must exist in every successor block");
            if matches!(op, OptOp::Jump(_, _)) { continue }

            let mut aggregated_effect = OpEffect::None;
            let mut program_position = None;
            let mut ksplang_ops_increment = None;
            let mut crosses_effect = false;

            for iid in instr_ids.iter() {
                let block = g.block_(iid.0);
                let instr = &block.instructions[&iid.1];
                assert_eq!(&instr.op, op);

                if matches!(op, OptOp::Checkpoint) && (
                    program_position.is_some_and(|p| p != instr.program_position) ||
                    ksplang_ops_increment.is_some_and(|p| p != instr.ksp_instr_count))
                {
                    // all checkpoints must point to the same location
                    continue 'candidate;
                }

                let (prior_effect, prior_checkpoint) = check_prior_effect(block, iid.1);
                crosses_effect |= prior_effect;

                if !can_hoist_from_block(g, block, iid.1, instr, prior_effect, prior_checkpoint) {
                    continue 'candidate;
                }

                aggregated_effect = OpEffect::worse_of(aggregated_effect, instr.effect);
                program_position = Some(instr.program_position);
                ksplang_ops_increment = Some(instr.ksp_instr_count);
            }

            let Some(insert_pos) = choose_insert_position(g, predecessor) else {
                continue;
            };

            let new_iid = g.make_instr_id_at(insert_pos, |_| false).unwrap();
            assert!(!g.block_(predecessor).instructions.contains_key(&new_iid.1));

            let new_out = if op.has_output() {
                let output_values: Vec<(ValueId, IRange, InstrId)> =
                    instr_ids.iter().filter_map(|id| {
                        let i = g.get_instruction_(*id);
                        if i.out.is_computed() {
                            let range = g.val_info_(i.out).range.clone();
                            Some((i.out, range, i.id))
                        } else { None }
                    }).collect();
                if output_values.is_empty() {
                    ValueId(0)
                } else {
                    let range = if crosses_effect {
                        // need to recompute, since we crossed an effectful operation which may have tightened the range
                        let in_ranges: Vec<IRange> = inputs.iter()
                            .map(|v| g.val_range_at(*v, new_iid))
                            .collect();
                        op.evaluate_range_quick(&in_ranges).unwrap_or(FULL_RANGE)
                    } else {
                        output_values.iter().map(|(_, range, _)| range.clone()).reduce(union_range).unwrap()
                    };
                    // TODO: is it better to re-use old value or create new one?
                    // let out_info = g.new_value();
                    let out_info = g.values.get_mut(output_values.iter().map(|(v, _, _)| v).min().unwrap()).unwrap();
                    out_info.assumptions.clear();
                    out_info.range = range.clone();
                    out_info.set_assigned_at(new_iid, op, inputs);
                    let new_out = out_info.id;
                    g.replace_values(output_values.iter().filter(|(v, _, _)| v != &new_out).map(|(v, _, _)| (*v, new_out)).collect());
                    // TODO: copy all assumes or is it invalid?
                    // preserve original value ranges:
                    for (val, orig_range, at) in output_values {
                        if orig_range != range {
                            g.add_assumption(val, at, Condition::True, orig_range);
                        }
                    }
                    new_out
               }
            } else {
                ValueId(0)
            };

            let hoisted_instr = OptInstr {
                id: new_iid,
                op: op.clone(),
                inputs: inputs.clone(),
                out: new_out,
                program_position: program_position.unwrap_or(usize::MAX),
                ksp_instr_count: ksplang_ops_increment.map_or(u32::MAX, |ctr| ctr + g.block_(predecessor).ksplang_instr_count),
                effect: aggregated_effect,
                annot: Annotations::default()
            };

            g.block_mut_(predecessor).instructions
                .insert(new_iid.instr_ix(), hoisted_instr.clone());

            for inp in hoisted_instr.iter_inputs() {
                g.mark_used_at(inp, new_iid);
            }

            for &iid in instr_ids.iter() {
                // g.remove_instruction(iid, false);
                let block = g.block_mut_(iid.block_id());
                let instr = block.instructions.remove(&iid.instr_ix()).unwrap();
                // remove from value-numbering to avoid crash on re-use
                if let Some(vn) = g.value_index.get_mut(&(instr.op.clone(), instr.inputs.clone())) {
                    vn.retain(|x| x.1 != iid);
                }
                // update used_at for the inputs
                for inp in instr.iter_inputs() {
                    if let Some(info) = g.values.get_mut(&inp) {
                        info.used_at.remove(&iid);
                        // if info.used_at.is_empty() { // TODO:
                        //     g.stack.poped_values.push(inp);
                        // }
                    }
                }
            }

            if g.conf.should_log(5) {
                println!("Hoisted {hoisted_instr} ({program_position:?}, {ksplang_ops_increment:?})");
            }

            hoisted_any = true;

            if hoisted_instr.op.is_terminal() {
                // remove all following instructions, mark following as unreachable
                let block = g.block_mut_(predecessor);
                block.outgoing_jumps.clear();
                block.instructions.split_off(&(hoisted_instr.id.1 + 1));
                for &f in &successors {
                    let b = g.block_mut_(f);
                    b.is_reachable = false;
                    assert!(1 >= b.incoming_jumps.len());
                    b.incoming_jumps.clear();
                }
            }
            continue 'main;
        }
        break;
    }

    hoisted_any
}

/// Hoists common computations from preceding blocks into the target block.
/// If multiple predecessors compute a block parameter using the same OptOp,
/// we can move the computation into the target block and pass the operands as block parameters instead.
/// Returns true if any hoisting was performed.
///
/// Requires that `g.current_block == target` and the block has no instructions yet.
pub fn hoist_down(g: &mut GraphBuilder, target: BlockId) -> bool {
    assert_eq!(g.current_block, target);

    let target_block = g.block_(target);
    assert_eq!(target_block.instructions.len(), 0);
    if target_block.incoming_jumps.len() < 2 || target_block.parameters.is_empty() {
        return false;
    }

    // All predecessors must unconditionally jump to this block
    for &jump_id in &target_block.incoming_jumps {
        if g.block_(jump_id.block_id()).outgoing_jumps.len() != 1 {
            return false;
        }
        assert_matches!(g.get_instruction_(jump_id).op, OptOp::Jump(Condition::True, _));
    }

    let mut candidates = find_down_hoist_candidates(g, target);
    if candidates.is_empty() {
        return false;
    }

    let incoming = g.block_(target).incoming_jumps.clone();
    let orig_params = g.block_(target).parameters.clone();
    let orig_args = g.get_instruction_(incoming[0]).inputs.clone();

    candidates.sort_by_key(|c| c.source_instrs[0].instr_ix());

    let removed_values: HashSet<ValueId> = candidates.iter().flat_map(|c| &c.out_vals).copied().collect();
    let param_indices: Vec<usize> =
        g.get_instruction_(incoming[0]).inputs.iter()
            .enumerate().filter_map(|(i, v)| removed_values.contains(v).then_some(i))
            .collect();

    g.block_mut_(target).parameters.remove_all(&param_indices);
    for &jump_id in &incoming {
        g.update_instr_inuts(jump_id, |jump| {
            jump.inputs.remove_all(&param_indices)
        });
    }

    for cand in candidates.iter().rev() {
        for &src_id in &cand.source_instrs {
            g.remove_instruction(src_id, false);
        }
    }

    let mut resolve_map: BTreeMap<ValueId, ValueId> = BTreeMap::new();

    for cand in candidates.iter() {
        if g.conf.should_log(10) {
            println!("  Processing candidate: {:?} out_vals={:?} ids={:?}", cand.op, cand.out_vals, cand.source_instrs);
            println!("    arg_values={:?}", cand.arg_values);
        }
        let new_inputs: SmallVec<[ValueId; 4]> = cand.arg_values.iter().enumerate().map(|(i, arg_vals)| {
            if let Some(resolved) = arg_vals.iter().filter_map(|v| resolve_map.get(v)).next() {
                if g.conf.should_log(10) {
                    println!("    arg_vals[{i}]={arg_vals:?} -> {resolved} (another hoisted operation)");
                }
                // all branches must resolve to the same value, otherwise we are fucked and actually cannot correctly construct the CFG
                assert!(arg_vals.iter().all(|v| resolve_map.get(v) == Some(resolved)),
                        "this shit should not happen anymore");
                *resolved
            } else {
                let new_param = find_or_create_param(g, target, &incoming, arg_vals);
                if g.conf.should_log(10) {
                    println!("    arg_vals[{i}]={arg_vals:?} -> {new_param} (new parameter)");
                }
                new_param
            }
        }).collect();

        let saved = g.assumed_program_position;
        g.assumed_program_position = Some(cand.program_position);
        let (new_val, _) = g.push_instr(cand.op.clone(), &new_inputs, false, None, Some(OpEffect::None));
        g.assumed_program_position = saved;

        if g.conf.should_log(5) {
            println!("Down-hoisted {:?} {:?} -> {}", cand.op, new_inputs, new_val);
        }

        for &out_val in &cand.out_vals {
            debug_assert!(out_val.is_computed());
            resolve_map.insert(out_val, new_val);
        }
    }

    // mark removed output parameters as replaced with the values computed by hoisted instructions
    let replacements: BTreeMap<ValueId, ValueId> =
        param_indices.iter()
            // .filter_map(|(&i, &arg)| resolve_map.get(&arg).map(|x| (orig_params[i], *x)))
            .map(|&i| (orig_params[i], resolve_map[&orig_args[i]]))
            .collect();
    // g.replaced_values.append(&mut replacements);
    g.replace_values(replacements);

    true
}

fn find_or_create_param(g: &mut GraphBuilder, target: BlockId, incoming: &[InstrId], values_per_pred: &[ValueId]) -> ValueId {
    for (pi, &param) in g.block_(target).parameters.iter().enumerate() {
        if incoming.iter().enumerate().all(|(pred_ix, &jid)| g.get_instruction_(jid).inputs[pi] == values_per_pred[pred_ix]) {
            if g.conf.should_log(10) {
                println!("    find_or_create_param: reusing param {param} at pi={pi} for values {values_per_pred:?}");
            }
            return param
        }
    }
    let range = values_per_pred.iter().map(|&v| g.val_range(v)).reduce(union_range).unwrap();
    let vi = g.new_value();
    vi.assigned_at = Some(InstrId(target, 0));
    vi.range = range;
    let new_param = vi.id;
    g.block_mut_(target).parameters.push(new_param);
    for (pred_ix, &jump_id) in incoming.iter().enumerate() {
        g.update_instr_inuts(jump_id, |jump| jump.inputs.push(values_per_pred[pred_ix]));
    }
    if g.conf.should_log(10) {
        println!("    find_or_create_param: created new param {new_param} for values {values_per_pred:?}");
    }
    new_param
}

#[derive(Debug)]
struct DownHoistCandidate {
    op: OptOp<ValueId>,
    out_vals: SmallVec<[ValueId; 4]>,
    source_instrs: SmallVec<[InstrId; 4]>,
    /// tranposed argument values
    arg_values: Vec<SmallVec<[ValueId; 4]>>,
    program_position: usize,
}

/// Find all down-hoistable candidates, including transitive ones.
/// Starts from block parameters and follows operand chains via a worklist.
fn find_down_hoist_candidates(g: &GraphBuilder, target: BlockId) -> Vec<DownHoistCandidate> {
    let target_block = g.block_(target);
    let incoming = &target_block.incoming_jumps;
    let params = &target_block.parameters;

    let mut queue: Vec<SmallVec<[ValueId; 4]>> = Vec::new();
    for (param_ix, _) in params.iter().enumerate() {
        let values: SmallVec<[ValueId; 4]> = incoming.iter().map(|&jump_id|
            g.get_instruction_(jump_id).inputs[param_ix]
        ).collect();
        queue.push(values);
    }

    let mut candidates = Vec::new();
    // mapping instrId -> candidate ID
    // value will be hoisted if it's used
    //  * in the same instructions as identified by candidate ID
    //  * in the same positions
    let mut allowed_instrs = BTreeMap::new();
    for x in incoming { allowed_instrs.insert(*x, usize::MAX); };

    while let Some(values) = queue.pop() {
        let Some(cand) = try_make_candidate(g, incoming, &allowed_instrs, &values) else {
            if g.conf.should_log(5) {
                println!("  Down-hoist candidate rejected: values={values:?}");
            }
            continue;
        };

        allowed_instrs.extend(cand.source_instrs.iter().map(|i| (*i, candidates.len())));

        for arg_vals in &cand.arg_values {
            if !all_equal(arg_vals.iter()) {
                queue.push(arg_vals.clone());
            }
        }

        candidates.push(cand);
    }

    candidates
}

/// Try to create a candidate from a set of values (one per predecessor).
/// Returns None if the values can't be down-hoisted.
fn try_make_candidate(g: &GraphBuilder,
                      incoming: &[InstrId],
                      allowed_instrs: &BTreeMap<InstrId, usize>,
                      values: &[ValueId],
) -> Option<DownHoistCandidate> {
    if !values.iter().any(ValueId::is_computed) {
        return None;
    }
    // All values must be computed by the same op in their respective predecessor blocks
    let mut source_instrs: SmallVec<[InstrId; 4]> = smallvec![];
    let mut op = None;
    for (pred_ix, &val) in values.iter().enumerate() {
        let defined_at = g.val_info_(val).assigned_at?;
        if defined_at.block_id() != incoming[pred_ix].block_id() { return None; }
        if defined_at.is_block_head() { return None; }
        let instr = g.get_instruction_(defined_at);
        debug_assert!(instr.op.has_output() && instr.out.is_computed() && instr.out == val);

        // TODO: can we do something with effectful instructions? Oo
        if !matches!(instr.effect, OpEffect::None | OpEffect::CtrIncrement) { return None; }

        debug_assert!(!matches!(instr.op, OptOp::Jump(_, _) | OptOp::Checkpoint | OptOp::Pop | OptOp::Push | OptOp::StackSwap | OptOp::StackRead | OptOp::Nop));

        match &op {
            None => {
                op = Some((instr.op.clone(), instr.inputs.len()));
            }
            Some((op, arity)) => {
                if &instr.op != op || instr.inputs.len() != *arity { return None; }
            }
        }
        source_instrs.push(defined_at);
    }
    let (op, arity) = op.unwrap();

    if source_instrs.iter().all(|s| allowed_instrs.contains_key(s)) { return None; }

    // value is only allowed to be used in
    // * the jump
    // * other instructions which are are hoisting already
    // and it must be used in exactly the same way in all blocks
    let signatures: Option<Vec<_>> = values.iter().map(|&v| downhoist_candidate_usage_signature(g, allowed_instrs, v)).collect();
    let Some(signatures) = signatures else {
        return None
    };
    if !all_equal(signatures.iter()) {
        return None
    }

    let mut arg_values: Vec<SmallVec<[ValueId; 4]>> = vec![smallvec![]; arity];
    for &src_id in &source_instrs {
        for (i, &inp) in g.get_instruction_(src_id).inputs.iter().enumerate() {
            arg_values[i].push(inp);
        }
    }

    let program_position = g.get_instruction_(source_instrs[0]).program_position;
    Some(DownHoistCandidate {
        op,
        source_instrs,
        out_vals: values.to_smallvec(),
        arg_values,
        program_position,
    })
}

fn downhoist_candidate_usage_signature(g: &GraphBuilder,
                                       allowed_instrs: &BTreeMap<InstrId, usize>,
                                       val: ValueId
) -> Option<Vec<(usize, usize)>> {
    let info = g.val_info_(val);
    let defined_at = info.assigned_at.unwrap();
    debug_assert!(!info.used_at.is_empty());

    let mut result: Vec<(usize, usize)> = vec![];
    for &used_at_id in &info.used_at {

        if used_at_id.block_id() != defined_at.block_id() { return None }

        let used_at = g.get_instruction_(used_at_id);

        let Some(&candidate_id) = allowed_instrs.get(&used_at_id) else {
            return None
        };
        debug_assert_eq!(candidate_id == usize::MAX, matches!(used_at.op, OptOp::Jump(_, _)));

        for (i, arg) in used_at.iter_inputs().enumerate() {
            if arg == val {
                result.push((candidate_id, i));
            }
        }
    }

    result.sort();
    Some(result)
}

/// Find instructions that appear in all blocks, grouped by (op, inputs).
/// Returns Vec of (op, inputs, Vec of InstrIds from each block)
fn get_common_instructions(
    g: &GraphBuilder,
    blocks: &[BlockId]
) -> Vec<(OptOp<ValueId>, SmallVec<[ValueId; 4]>, SmallVec<[InstrId; 4]>)> {
    assert!(!blocks.is_empty());

    // find smallest block
    let starter_block = blocks.iter()
        .enumerate()
        .min_by_key(|&(_, &block_id)| g.block_(block_id).instructions.len())
        .map(|(idx, _)| idx)
        .unwrap();

    // Map: (op, inputs) -> SmallVec of Option<InstrIds> (one per block)
    let mut instruction_map: HashMap<(OptOp<ValueId>, SmallVec<[ValueId; 4]>), SmallVec<[Option<u32>; 4]>> = HashMap::default();

    let smallest_block = g.block_(blocks[starter_block]);
    for (_instr_idx, instr) in smallest_block.instructions.iter() {
        let key = (instr.op.clone(), instr.inputs.clone());
        instruction_map.insert(key, smallvec![None; blocks.len()]);
    }

    for (ix, &block_id) in blocks.iter().enumerate() {
        let block = g.block_(block_id);
        for (&instr_idx, instr) in block.instructions.iter() {
            let key = (instr.op.clone(), instr.inputs.clone());

            if let Some(entry) = instruction_map.get_mut(&key) {
                entry[ix].get_or_insert(instr_idx);
            }
        }
    }

    // filter instructions that appear in ALL blocks
    instruction_map.into_iter()
        .filter_map(|((op, inputs), instr_indices)| {
            let res: Option<SmallVec<[InstrId; _]>> =
                instr_indices.into_iter().enumerate()
                    .map(|(ix, instr)| instr.map(|instr| InstrId(blocks[ix], instr)))
                    .collect();
            res.map(|instr_indices| (op, inputs, instr_indices))
        })
        .collect()
}

fn choose_insert_position(
    g: &GraphBuilder,
    predecessor: BlockId,
) -> Option<BeforeOrAfter<InstrId>> {
    let pred_block = g.block_(predecessor);

    let anchor = pred_block
        .instructions
        .iter().rev()
        .filter(|(_ix, instr)| !matches!(instr.op, OptOp::Jump(..)))
        .map(|(&ix, _)| ix)
        .next()
        .unwrap_or(0);

    Some(BeforeOrAfter::After(InstrId(predecessor, anchor)))
}

fn check_prior_effect(block: &BasicBlock, instr_idx: u32) -> (bool, bool) {
    let mut prior_effect = false;
    let mut prior_checkpoint = false;
    for (_, prior) in block.instructions.range(..instr_idx) {
        prior_effect = prior_effect || prior.effect != OpEffect::None;
        prior_checkpoint = prior_checkpoint || matches!(prior.op, OptOp::Checkpoint);
    }
    (prior_effect, prior_checkpoint)
}

fn can_hoist_from_block(
    g: &GraphBuilder,
    block: &BasicBlock,
    _instr_idx: u32,
    instr: &OptInstr,
    prior_effect: bool,
    prior_checkpoint: bool,
) -> bool {
    if !matches!(instr.op, OptOp::Checkpoint) && instr.op.worst_case_effect() == OpEffect::None {
        return true
    }


    if !prior_effect && !prior_checkpoint {
        return true
    }
    if matches!(instr.op, OptOp::KsplangOpsIncrement(_)) {
        return !prior_checkpoint
    }
    if matches!(instr.op, OptOp::Checkpoint) {
        return !prior_checkpoint && !prior_effect
    }
    if !matches!(instr.effect, OpEffect::None | OpEffect::MayFail) {
        return !prior_effect && !prior_checkpoint
    }
    if matches!(instr.op, OptOp::Assert(_, OperationError::Unreachable)) {
        // always panics, "error_is_deopt" doesn't apply
        return !prior_effect
    }


    // check the effect at start of the block
    //  - we need to be sure that the effect wasn't just masked by a preceeding
    //  - even though we move it into the previous block, checking start of the current block is sufficient
    //    For example:
    //      if (a != 0) { StackWrite(0, a); b / a } else { b / a }
    //    We can safely hoist (b / a), because:
    //      in block1: it cannot have an effect
    //      in block2: it's the first instruction, so moving the effect before branch does not change anything
    let op_ranges: Vec<_> = instr.inputs.iter().map(|&v| g.val_range_at(v, InstrId(block.id, 0))).collect();
    let effect_hoisted = instr.op.effect_based_on_ranges(&op_ranges);

    match effect_hoisted {
        OpEffect::None => true,
        // TODO: should be valid, but is it actually a good idea?
        //  2 weeks later: no, it's not (for now), because the instruction didn't have any effect previously
        //                 and we will not correctly assign it the correct effect=OpEffect::MayFail
        // OpEffect::MayFail => if g.conf.error_as_deopt { true } else { !prior_effect },
        OpEffect::MayFail if g.conf.error_as_deopt && instr.effect == OpEffect::MayFail => true,
        // always ok to swap error and checkpoint, since error is very unlikely
        OpEffect::MayFail => !prior_effect,
        _ => false
    }
}

// fn toposort_candidates(g: &GraphBuilder, candidates: Vec<DownHoistCandidate>) -> Vec<DownHoistCandidate> {
//     if candidates.len() <= 1 { return candidates; }
//
//     let instr: Vec<&OptInstr> = candidates.iter().map(|c| g.get_instruction_(c.source_instrs[0])).collect();
//
//     let val_to_ci: HashMap<ValueId, usize> =
//         instr.iter().enumerate().map(|(ix, instr)| (instr.out, ix)).collect();
//
//     let n = candidates.len();
//     let mut dependents: Vec<SmallVec<[usize; 4]>> = vec![smallvec![]; n];
//     let mut in_deg: Vec<usize> = vec![0; n];
//     for (ci, instr) in instr.iter().enumerate() {
//         for &inp in instr.inputs.iter() {
//             if let Some(&dep) = val_to_ci.get(&inp) {
//                 assert_ne!(dep, ci);
//                 dependents[dep].push(ci);
//                 in_deg[ci] += 1;
//             }
//         }
//     }
//
//     let mut candidates: Vec<Option<DownHoistCandidate>> = candidates.into_iter().map(Some).collect();
//     let mut stack: Vec<usize> = (0..n).filter(|&i| in_deg[i] == 0).collect();
//     let mut result: Vec<DownHoistCandidate> = Vec::with_capacity(n);
//
//     while let Some(ci) = stack.pop() {
//         result.push(candidates[ci].take().unwrap());
//         for &other in &dependents[ci] {
//             in_deg[other] -= 1;
//             if in_deg[other] == 0 { stack.push(other); }
//         }
//     }
//     assert_eq!(result.len(), n);
//     result
// }



#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{cfg::GraphBuilder, ops::{OptOp, ValueId}, osmibytecode::Condition};

    // Test helpers to reduce verbosity
    fn push_branch(g: &mut GraphBuilder, cond: Condition<ValueId>, t: BlockId, f: BlockId) {
        g.push_instr(OptOp::Jump(cond, t), &[], false, None, None);
        g.push_instr(OptOp::Jump(Condition::True, f), &[], false, None, None);
    }

    fn push_with_deopt(g: &mut GraphBuilder, block: BlockId, op: OptOp<ValueId>, args: &[ValueId]) {
        g.switch_to_block(block, 0, vec![]);
        if args.is_empty() {
            g.push_instr(op, &[], false, None, None);
        } else {
            g.push_instr(op, args, false, None, None);
        }
        g.push_instr(OptOp::deopt_always(), &[], false, None, None);
    }

    fn graph_with_param(range: std::ops::RangeInclusive<i64>) -> (GraphBuilder, ValueId) {
        let mut g = GraphBuilder::new(0);
        let val_info = g.new_value();
        val_info.range = range;
        let param = val_info.id;
        g.stack.push(param);
        (g, param)
    }

    #[test]
    fn test_hoist_pure_instruction() {
        let (mut g, param) = graph_with_param(0..=10);

        // Create branching structure:
        // bb0: if param == 0 goto bb1 else goto bb2
        // bb1: add param, 1; ...
        // bb2: add param, 1; ...

        let bb1 = g.new_block(1, true, vec![]);
        let bb1_id = bb1.id;
        let bb2 = g.new_block(2, true, vec![]);
        let bb2_id = bb2.id;

        let bb0_id = g.current_block;
        push_branch(&mut g, Condition::EqConst(param, 0), bb1_id, bb2_id);

        push_with_deopt(&mut g, bb1_id, OptOp::Add, &[param, ValueId::C_ONE]);
        push_with_deopt(&mut g, bb2_id, OptOp::Add, &[param, ValueId::C_ONE]);

        // Both blocks should have the add instruction
        assert_eq!(g.block_(bb1_id).instructions.len(), 2);
        assert_eq!(g.block_(bb2_id).instructions.len(), 2);

        // Try to hoist
        let hoisted = hoist_up(&mut g, bb0_id);

        assert!(hoisted, "Should have hoisted the common instructions");

        // Both Add and deopt should be hoisted, leaving bb1 and bb2 empty
        assert_eq!(g.block_(bb1_id).instructions.len(), 0, "bb1 should be empty after hoisting");
        assert_eq!(g.block_(bb2_id).instructions.len(), 0, "bb2 should be empty after hoisting");

        // bb0 should have Add + deopt, and the old conditional jump should be removed (unreachable after deopt)
        let bb0_instrs: Vec<_> = g.block_(bb0_id).instructions.values().collect();
        assert!(bb0_instrs.iter().any(|i| matches!(i.op, OptOp::Add)), "Should have Add");
        assert!(bb0_instrs.iter().any(|i| matches!(i.op, OptOp::DeoptAssert(_))), "Should have deopt");
    }

    #[test]
    fn test_hoist_pop_with_nested_branches() {
        // Test case similar to the user's example:
        // bb0 -> bb1, bb2
        // bb1 -> Pop, then branches
        // bb2 -> Pop, then branches
        // Only the Pop should be hoisted to bb0 (jumps stay in successors)

        let mut g = GraphBuilder::new(0);
        let v1 = g.new_value().id;

        let bb1 = g.new_block(1, false, vec![]);
        let bb1_id = bb1.id;
        let bb2 = g.new_block(2, false, vec![]);
        let bb2_id = bb2.id;

        // bb0: branches based on v1
        let bb0_id = g.current_block;
        push_branch(&mut g, Condition::GtConst(v1, 4), bb1_id, bb2_id);

        // Create bb3 and bb4 for the second level of branches
        let bb3 = g.new_block(3, false, vec![]);
        let bb3_id = bb3.id;
        let bb4 = g.new_block(4, false, vec![]);
        let bb4_id = bb4.id;

        // bb1: Pop, then branch
        g.switch_to_block(bb1_id, 0, vec![]);
        g.push_instr(OptOp::Pop, &[], false, None, None);
        push_branch(&mut g, Condition::GtConst(v1, 3), bb3_id, bb4_id);

        // bb2: Pop (same as bb1), then branch
        g.switch_to_block(bb2_id, 0, vec![]);
        g.push_instr(OptOp::Pop, &[], false, None, None);
        push_branch(&mut g, Condition::GtConst(v1, 3), bb3_id, bb4_id);

        // Seal the blocks in the order they would be sealed during compilation
        g.seal_block(bb1_id);
        g.seal_block(bb2_id);

        // Both bb1 and bb2 should have 3 instructions (Pop + 2 jumps)
        assert_eq!(g.block_(bb1_id).instructions.len(), 3);
        assert_eq!(g.block_(bb2_id).instructions.len(), 3);

        // Try to hoist from bb0 (should hoist the Pop)
        let hoisted = hoist_up(&mut g, bb0_id);

        assert!(hoisted, "Should have hoisted instructions");

        // Only Pop should be hoisted; jumps remain in successors
        assert_eq!(g.block_(bb1_id).instructions.len(), 2, "bb1 should have its two jumps left");
        assert_eq!(g.block_(bb2_id).instructions.len(), 2, "bb2 should have its two jumps left");
        assert!(g.block_(bb1_id).instructions.values().all(|instr| matches!(instr.op, OptOp::Jump(..))), "bb1 should only contain jumps");
        assert!(g.block_(bb2_id).instructions.values().all(|instr| matches!(instr.op, OptOp::Jump(..))), "bb2 should only contain jumps");

        // bb0 should now have the Pop inserted before its existing jumps
        let bb0_instrs: Vec<_> = g.block_(bb0_id).instructions.values().collect();
        assert!(bb0_instrs.iter().any(|i| matches!(i.op, OptOp::Pop)), "Should have Pop");
        assert!(bb0_instrs.iter().filter(|i| matches!(i.op, OptOp::Jump(..))).count() >= 2, "Original jumps should remain");
        let targets: Vec<_> = g.block_(bb0_id).outgoing_jumps.iter().map(|(_, target)| *target).collect();
        assert!(targets.contains(&bb1_id) && targets.contains(&bb2_id), "Original branch targets should remain");
    }

    #[test]
    fn test_hoist_non_first_instruction() {
        // Test that we can hoist instructions that are not at the first position
        // bb0 -> bb1, bb2
        // bb1 -> Pop, Add, deopt
        // bb2 -> Pop, Add, deopt
        // Both Pop and Add should be hoisted

        let (mut g, param) = graph_with_param(0..=10);

        let bb1 = g.new_block(1, true, vec![]);
        let bb1_id = bb1.id;
        let bb2 = g.new_block(2, true, vec![]);
        let bb2_id = bb2.id;

        let bb0_id = g.current_block;
        push_branch(&mut g, Condition::EqConst(param, 0), bb1_id, bb2_id);

        // bb1: Pop, Max, deopt
        g.switch_to_block(bb1_id, 0, vec![]);
        g.push_instr(OptOp::Pop, &[], false, None, None);
        push_with_deopt(&mut g, bb1_id, OptOp::Max, &[param, ValueId::C_ONE]);

        // bb2: different order - Add, Max, deopt
        g.switch_to_block(bb2_id, 0, vec![]);
        g.push_instr(OptOp::Add, &[param, ValueId::C_ONE], false, None, Some(OpEffect::None));
        push_with_deopt(&mut g, bb2_id, OptOp::Max, &[param, ValueId::C_ONE]);

        assert_eq!(g.block_(bb1_id).instructions.len(), 3);
        assert_eq!(g.block_(bb2_id).instructions.len(), 3);

        let hoisted = hoist_up(&mut g, bb0_id);

        assert!(hoisted, "Should have hoisted Max");

        // Should have hoisted Max (common instruction), leaving Pop + deopt in bb1 and Add + deopt in bb2
        assert_eq!(g.block_(bb1_id).instructions.len(), 2, "bb1 should have Pop + deopt left");
        assert_eq!(g.block_(bb2_id).instructions.len(), 2, "bb2 should have Add + deopt left");

        // bb0 should have Max hoisted before jumps
        let bb0_instrs: Vec<_> = g.block_(bb0_id).instructions.values().collect();
        assert!(bb0_instrs.iter().any(|i| matches!(i.op, OptOp::Max)), "Should have Max");
    }

    #[test]
    fn test_hoist_jumps() {
        // Test that we can hoist jumps, including conditional and unconditional
        // bb0 -> bb1, bb2
        // bb1 -> Pop, Jump(cond, bb3), Jump(true, bb4)
        // bb2 -> Pop, Jump(cond, bb3), Jump(true, bb4)
        // Only Pop should be hoisted; jumps stay in successors

        let mut g = GraphBuilder::new(0);
        let v1 = g.new_value().id;

        let bb1 = g.new_block(1, false, vec![]);
        let bb1_id = bb1.id;
        let bb2 = g.new_block(2, false, vec![]);
        let bb2_id = bb2.id;
        let bb3 = g.new_block(3, false, vec![]);
        let bb3_id = bb3.id;
        let bb4 = g.new_block(4, false, vec![]);
        let bb4_id = bb4.id;

        let bb0_id = g.current_block;
        push_branch(&mut g, Condition::GtConst(v1, 5), bb1_id, bb2_id);

        // bb1: Pop, then jumps to bb3 or bb4
        g.switch_to_block(bb1_id, 0, vec![]);
        g.push_instr(OptOp::Pop, &[], false, None, None);
        push_branch(&mut g, Condition::GtConst(v1, 3), bb3_id, bb4_id);

        // bb2: Pop, then same jumps
        g.switch_to_block(bb2_id, 0, vec![]);
        g.push_instr(OptOp::Pop, &[], false, None, None);
        push_branch(&mut g, Condition::GtConst(v1, 3), bb3_id, bb4_id);

        g.seal_block(bb1_id);
        g.seal_block(bb2_id);

        assert_eq!(g.block_(bb1_id).instructions.len(), 3);
        assert_eq!(g.block_(bb2_id).instructions.len(), 3);

        let hoisted = hoist_up(&mut g, bb0_id);

        assert!(hoisted, "Should have hoisted instructions");

        // Pop should hoist, leaving two jumps in each successor
        assert_eq!(g.block_(bb1_id).instructions.len(), 2, "bb1 should have jumps left");
        assert_eq!(g.block_(bb2_id).instructions.len(), 2, "bb2 should have jumps left");
        assert!(g.block_(bb1_id).instructions.values().all(|instr| matches!(instr.op, OptOp::Jump(..))), "bb1 should only have jumps");
        assert!(g.block_(bb2_id).instructions.values().all(|instr| matches!(instr.op, OptOp::Jump(..))), "bb2 should only have jumps");

        let bb0_instrs: Vec<_> = g.block_(bb0_id).instructions.values().collect();
        assert!(bb0_instrs.iter().any(|i| matches!(i.op, OptOp::Pop)), "Should have Pop");
        let jump_count = bb0_instrs.iter().filter(|i| matches!(i.op, OptOp::Jump(..))).count();
        assert_eq!(jump_count, 2, "Original two jumps should remain in predecessor");

        let targets: Vec<_> = g.block_(bb0_id).outgoing_jumps.iter().map(|(_, target)| *target).collect();
        assert!(targets.contains(&bb1_id) && targets.contains(&bb2_id), "Predecessor should still branch to bb1 and bb2");
    }

    #[test]
    fn test_hoist_pop_after_pure_instructions() {
        // Test that Pop can be hoisted even when preceded by effect-free instructions
        // bb0 -> bb1, bb2
        // bb1 -> Add, Pop, deopt
        // bb2 -> Add, Pop, deopt
        // Pop should be hoisted (Add is not common due to different input order or similar)
        
        let (mut g, param) = graph_with_param(0..=10);
        
        let bb1 = g.new_block(1, true, vec![]);
        let bb1_id = bb1.id;
        let bb2 = g.new_block(2, true, vec![]);
        let bb2_id = bb2.id;
        
        let bb0_id = g.current_block;
        push_branch(&mut g, Condition::EqConst(param, 0), bb1_id, bb2_id);
        
        // bb1: Max (effect-free), Pop (effectful), Add (may fail)
        g.switch_to_block(bb1_id, 0, vec![]);
        g.push_instr(OptOp::Max, &[param, ValueId::C_ONE], false, None, Some(OpEffect::None));
        let val1 = g.push_instr(OptOp::Pop, &[], false, None, None).0;
        g.push_instr(OptOp::Add, &[param, val1], false, None, None);
        
        // bb2: Min (effect-free), Pop (effectful), Mul (may fail)
        g.switch_to_block(bb2_id, 0, vec![]);
        g.push_instr(OptOp::Min, &[param, ValueId::C_ONE], false, None, Some(OpEffect::None));
        let val2 = g.push_instr(OptOp::Pop, &[], false, None, None).0;
        g.push_instr(OptOp::Mul, &[param, val2], false, None, None);
        
        assert_eq!(g.block_(bb1_id).instructions.len(), 3, "{g}");
        assert_eq!(g.block_(bb2_id).instructions.len(), 3, "{g}");
        println!("Before hoisting:\n{}", g);
        
        let hoisted = hoist_up(&mut g, bb0_id);
        println!("After hoisting:\n{}", g);
        
        assert!(hoisted, "Should have hoisted common instructions");
        
        // Pop should be hoisted even though preceded by effect-free instructions
        // After hoisting Pop, bb1 should have Max and Add left, bb2 should have Min and Mul left
        let bb1_len = g.block_(bb1_id).instructions.len();
        let bb2_len = g.block_(bb2_id).instructions.len();
        assert_eq!(bb1_len, 2, "bb1 should have Max and Add left, got {} instructions", bb1_len);
        assert_eq!(bb2_len, 2, "bb2 should have Min and Mul left, got {} instructions", bb2_len);
        
        // bb0 should have Pop hoisted
        let bb0_instrs: Vec<_> = g.block_(bb0_id).instructions.values().collect();
        assert!(bb0_instrs.iter().any(|i| matches!(i.op, OptOp::Pop)), "Should have Pop hoisted");
    }

    #[test]
    fn test_hoist_down_basic() {
        // bb0 -> bb1, bb2 -> bb3
        // bb1: v_add1 = Add(param, C2); Jump(true, bb3, [v_add1])
        // bb2: v_add2 = Add(param, C2); Jump(true, bb3, [v_add2])
        // bb3: phi = block parameter
        // After hoist_down(bb3): Add should be pulled into bb3

        let (mut g, param) = graph_with_param(0..=10);

        let bb1_id = g.new_block(1, true, vec![]).id;
        let bb2_id = g.new_block(2, true, vec![]).id;
        push_branch(&mut g, Condition::EqConst(param, 0), bb1_id, bb2_id);

        let phi = g.new_value().id;
        let bb3_id = g.new_block(0, true, vec![phi]).id;

        g.switch_to_block(bb1_id, 0, vec![]);
        let (v_add1, _) = g.push_instr(OptOp::Add, &[param, ValueId::C_TWO], false, None, Some(OpEffect::None));
        let jump1 = g.push_instr(OptOp::Jump(Condition::True, bb3_id), &[v_add1], false, None, None).1.unwrap().id;
        g.block_mut_(bb3_id).incoming_jumps.push(jump1);
        g.block_mut_(bb3_id).predecessors.insert(bb1_id);

        g.switch_to_block(bb2_id, 0, vec![]);
        let (v_add2, _) = g.push_instr(OptOp::Add, &[param, ValueId::C_TWO], false, None, Some(OpEffect::None));
        let jump2 = g.push_instr(OptOp::Jump(Condition::True, bb3_id), &[v_add2], false, None, None).1.unwrap().id;
        g.block_mut_(bb3_id).incoming_jumps.push(jump2);
        g.block_mut_(bb3_id).predecessors.insert(bb2_id);

        g.switch_to_block(bb3_id, 0, vec![]);
        println!("Before hoist_down:\n{g}");

        let hoisted = hoist_down(&mut g, bb3_id);
        println!("After hoist_down:\n{g}");

        assert!(hoisted);

        assert!(g.block_(bb1_id).instructions.values().all(|i| matches!(i.op, OptOp::Jump(..))));
        assert!(g.block_(bb2_id).instructions.values().all(|i| matches!(i.op, OptOp::Jump(..))));

        assert!(g.block_(bb3_id).instructions.values().any(|i| matches!(i.op, OptOp::Add)));
    }

    #[test]
    fn test_hoist_down_transitive() {
        // bb0 -> bb1, bb2 -> bb3
        // bb1: v_max1 = Max(param, C2); v_mul1 = Mul(v_max1, C100); Jump(true, bb3, [v_mul1])
        // bb2: v_max2 = Max(param, C2); v_mul2 = Mul(v_max2, C100); Jump(true, bb3, [v_mul2])
        // bb3: phi = block parameter
        // After hoist_down(bb3): BOTH Max and Mul should be pulled into bb3

        let (mut g, param) = graph_with_param(0..=10);
        let c100 = g.store_constant(100);

        let phi = g.new_value().id;

        let bb1_id = g.new_block(1, true, vec![]).id;
        let bb2_id = g.new_block(2, true, vec![]).id;
        push_branch(&mut g, Condition::EqConst(param, 0), bb1_id, bb2_id);

        let bb3_id = g.new_block(3, true, vec![phi]).id;

        g.switch_to_block(bb1_id, 0, vec![]);
        let (v_max1, _) = g.push_instr(OptOp::Max, &[param, ValueId::C_TWO], false, None, Some(OpEffect::None));
        let (v_mul1, _) = g.push_instr(OptOp::Mul, &[v_max1, c100], false, None, Some(OpEffect::None));
        let jump1 = g.push_instr(OptOp::Jump(Condition::True, bb3_id), &[v_mul1], false, None, None).1.unwrap().id;
        g.block_mut_(bb3_id).incoming_jumps.push(jump1);
        g.block_mut_(bb3_id).predecessors.insert(bb1_id);

        g.switch_to_block(bb2_id, 0, vec![]);
        let (v_max2, _) = g.push_instr(OptOp::Max, &[param, ValueId::C_TWO], false, None, Some(OpEffect::None));
        let (v_mul2, _) = g.push_instr(OptOp::Mul, &[v_max2, c100], false, None, Some(OpEffect::None));
        let jump2 = g.push_instr(OptOp::Jump(Condition::True, bb3_id), &[v_mul2], false, None, None).1.unwrap().id;
        g.block_mut_(bb3_id).incoming_jumps.push(jump2);
        g.block_mut_(bb3_id).predecessors.insert(bb2_id);

        g.switch_to_block(bb3_id, 0, vec![]);
        println!("Before hoist_down:\n{g}");

        let hoisted = hoist_down(&mut g, bb3_id);
        println!("After hoist_down:\n{g}");

        assert!(hoisted);
        assert_eq!(g.block_(bb1_id).instructions.len(), 1);
        assert_eq!(g.block_(bb2_id).instructions.len(), 1);

        let bb3_instrs: Vec<_> = g.block_(bb3_id).instructions.values().collect();
        let max_pos = bb3_instrs.iter().position(|i| matches!(i.op, OptOp::Max)).unwrap();
        let mul_pos = bb3_instrs.iter().position(|i| matches!(i.op, OptOp::Mul)).unwrap();

        assert!(max_pos < mul_pos, "Max should come before Mul in bb3");

        let max_out = bb3_instrs[max_pos].out;
        let mul_instr = &bb3_instrs[mul_pos];
        assert!(mul_instr.inputs.contains(&max_out));
        assert!(mul_instr.inputs.contains(&c100));
    }
}
