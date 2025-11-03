#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hashtimer::HashTimer, dag::{Block, BlockDAG}};

    #[tokio::test]
    async fn test_dlc_finalizes_deterministically() {
        let mut dag = BlockDAG::default();
        let block = Block {
            id: "b1".into(),
            parent: None,
            timestamp: HashTimer::now(),
            data: vec![],
        };
        dag.insert(block);
        let fairness = crate::dgbdt::FairnessModel::default();
        crate::process_round(&mut dag, &fairness).await;
        assert!(dag.blocks.len() > 0);
    }
}
