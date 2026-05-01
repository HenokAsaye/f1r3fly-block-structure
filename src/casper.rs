//! Casper GHOST fork-choice helper.

use std::collections::HashSet;

use crate::storage::{BlockStore, StoreError};
use crate::types::{BlockHash, Bond};

/// Implements the GHOST (Greediest Heaviest Observed SubTree) fork-choice rule.
pub struct GhostForkChoice;

impl GhostForkChoice {
    /// Run GHOST from genesis to find the canonical tip.
    ///
    /// Returns `StoreError` for storage access failures.
    pub async fn find_tip<S: BlockStore>(
        store: &S,
        bonds: &[Bond],
    ) -> Result<Option<BlockHash>, StoreError> {
        let genesis = match store.get_genesis().await? {
            Some(block) => block,
            None => return Ok(None),
        };
        let latest_messages = store.get_all_latest_messages().await?;

        let mut current = genesis.block_hash;
        loop {
            let children = store.get_children(&current).await?;
            if children.is_empty() {
                return Ok(Some(current));
            }

            let mut best_child = None;
            let mut best_weight = i64::MIN;
            for child in children {
                let weight = Self::subtree_weight(store, &child, &latest_messages, bonds).await?;
                let is_better = weight > best_weight
                    || (weight == best_weight && best_child.is_none_or(|c| child > c));
                if is_better {
                    best_weight = weight;
                    best_child = Some(child);
                }
            }

            match best_child {
                Some(next) => current = next,
                None => return Ok(Some(current)),
            }
        }
    }

    /// Compute total stake weight in subtree rooted at `root`.
    async fn subtree_weight<S: BlockStore>(
        store: &S,
        root: &BlockHash,
        latest_messages: &[(Vec<u8>, BlockHash)],
        bonds: &[Bond],
    ) -> Result<i64, StoreError> {
        let mut total = 0i64;
        for (validator, latest_hash) in latest_messages {
            if Self::is_in_subtree(store, latest_hash, root).await? {
                total += bond_stake(bonds, validator);
            }
        }
        Ok(total)
    }

    /// Check if `candidate` is in the subtree rooted at `root`.
    async fn is_in_subtree<S: BlockStore>(
        store: &S,
        candidate: &BlockHash,
        root: &BlockHash,
    ) -> Result<bool, StoreError> {
        if candidate == root {
            return Ok(true);
        }

        let mut stack = vec![*candidate];
        let mut visited = HashSet::new();

        while let Some(current) = stack.pop() {
            if !visited.insert(current) {
                continue;
            }
            let block = match store.get(&current).await? {
                Some(block) => block,
                None => continue,
            };
            for parent in block.header.parents_hash_list {
                if parent == *root {
                    return Ok(true);
                }
                stack.push(parent);
            }
        }

        Ok(false)
    }
}

fn bond_stake(bonds: &[Bond], validator: &[u8]) -> i64 {
    bonds
        .iter()
        .find(|bond| bond.validator == validator)
        .map(|bond| bond.stake)
        .unwrap_or(0)
}
