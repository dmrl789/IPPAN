//! Deterministic Learning Consensus (DLC)
//! Combines HashTimer™, BlockDAG, and D-GBDT fairness for deterministic finality.

pub mod hashtimer;
pub mod dag;
pub mod dgbdt;
pub mod verifier;
pub mod reputation;
pub mod emission;
pub mod bond;
#[cfg(test)]
mod tests;

use hashtimer::HashTimer;
use dag::BlockDAG;
use verifier::{VerifierSet, VerifiedBlock};
use dgbdt::FairnessModel;

pub fn init_dlc() {
    log::info!("Initializing Deterministic Learning Consensus (DLC)...");
}

/// Process one consensus round deterministically
pub async fn process_round(dag: &mut BlockDAG, fairness: &FairnessModel) {
    let round_time = HashTimer::now();
    let verifiers = VerifierSet::select(fairness, round_time.hash.clone());
    let blocks = verifiers.collect_blocks(dag.pending());
    for block in blocks {
        if verifiers.validate(&block) {
            let verified = VerifiedBlock(block);
            dag.insert(verified.0);
        }
    }
    dag.finalize_round(round_time);
    log::info!("DLC round finalized at {:?}", round_time);
}
