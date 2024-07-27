// File: crates/icn_consensus/src/lib.rs

use icn_common::{IcnResult, IcnError, Block, Transaction};
use std::collections::HashMap;
use log::{info, warn, error};
use std::sync::{Arc, RwLock};

pub struct PoCConsensus {
    threshold: f64,
    quorum: f64,
    validators: HashMap<String, Validator>,
    pending_blocks: Vec<Block>,
    blockchain: Arc<RwLock<Vec<Block>>>,
}

struct Validator {
    reputation: f64,
    // Add more validator-related information as needed
}

impl PoCConsensus {
    pub fn new(threshold: f64, quorum: f64) -> IcnResult<Self> {
        if threshold <= 0.0 || threshold > 1.0 || quorum <= 0.0 || quorum > 1.0 {
            return Err(IcnError::Consensus("Invalid threshold or quorum value".into()));
        }

        Ok(PoCConsensus {
            threshold,
            quorum,
            validators: HashMap::new(),
            pending_blocks: Vec::new(),
            blockchain: Arc::new(RwLock::new(Vec::new())),
        })
    }

    pub fn start(&self) -> IcnResult<()> {
        info!("PoC Consensus mechanism started");
        Ok(())
    }

    pub fn stop(&self) -> IcnResult<()> {
        info!("PoC Consensus mechanism stopped");
        Ok(())
    }

    pub fn add_validator(&mut self, id: String, initial_reputation: f64) -> IcnResult<()> {
        if initial_reputation < 0.0 || initial_reputation > 1.0 {
            return Err(IcnError::Consensus("Invalid initial reputation".into()));
        }
        self.validators.insert(id, Validator { reputation: initial_reputation });
        Ok(())
    }

    pub fn remove_validator(&mut self, id: &str) -> IcnResult<()> {
        self.validators.remove(id);
        Ok(())
    }

    pub fn process_new_block(&mut self, block: Block) -> IcnResult<()> {
        self.pending_blocks.push(block);
        self.try_reach_consensus()
    }

    fn try_reach_consensus(&mut self) -> IcnResult<()> {
        let total_reputation: f64 = self.validators.values().map(|v| v.reputation).sum();
        let quorum_reputation = total_reputation * self.quorum;

        let mut blocks_to_retain = Vec::new();

        for block in &self.pending_blocks {
            let mut votes_for = 0.0;
            let mut total_votes = 0.0;

            for validator in self.validators.values() {
                if self.validate_block(block)? {
                    votes_for += validator.reputation;
                }
                total_votes += validator.reputation;

                if total_votes >= quorum_reputation {
                    if votes_for / total_votes >= self.threshold {
                        self.add_block_to_chain(block.clone())?;
                    } else {
                        return Err(IcnError::Consensus("Block rejected by consensus".into()));
                    }
                }
            }
            blocks_to_retain.push(block.clone());
        }

        self.pending_blocks.retain(|b| blocks_to_retain.contains(b));

        Ok(())
    }

    fn validate_block(&self, block: &Block) -> IcnResult<bool> {
        if block.index == 0 {
            return Ok(true); // Genesis block is always valid
        }

        let blockchain = self.blockchain.read().map_err(|_| IcnError::Consensus("Failed to read blockchain".into()))?;
        let previous_block = blockchain.last().ok_or_else(|| IcnError::Consensus("No previous block found".into()))?;

        if block.previous_hash != previous_block.hash {
            return Ok(false);
        }

        if block.hash != block.calculate_hash() {
            return Ok(false);
        }

        for transaction in &block.transactions {
            if !self.validate_transaction(transaction)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn validate_transaction(&self, transaction: &Transaction) -> IcnResult<bool> {
        if transaction.amount <= 0.0 {
            return Ok(false);
        }

        Ok(true)
    }

    fn add_block_to_chain(&mut self, block: Block) -> IcnResult<()> {
        let mut blockchain = self.blockchain.write().map_err(|_| IcnError::Consensus("Failed to write to blockchain".into()))?;
        blockchain.push(block);
        Ok(())
    }

    pub fn get_blockchain(&self) -> IcnResult<Vec<Block>> {
        let blockchain = self.blockchain.read().map_err(|_| IcnError::Consensus("Failed to read blockchain".into()))?;
        Ok(blockchain.clone())
    }

    pub fn update_validator_reputation(&mut self, id: &str, new_reputation: f64) -> IcnResult<()> {
        if new_reputation < 0.0 || new_reputation > 1.0 {
            return Err(IcnError::Consensus("Invalid reputation value".into()));
        }

        if let Some(validator) = self.validators.get_mut(id) {
            validator.reputation = new_reputation;
            Ok(())
        } else {
            Err(IcnError::Consensus("Validator not found".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_block(index: u64, previous_hash: &str) -> Block {
        Block {
            index,
            timestamp: Utc::now().timestamp(),
            transactions: vec![],
            previous_hash: previous_hash.to_string(),
            hash: format!("test_hash_{}", index),
        }
    }

    #[test]
    fn test_poc_consensus_creation() {
        let consensus = PoCConsensus::new(0.66, 0.51);
        assert!(consensus.is_ok());
    }

    #[test]
    fn test_add_and_remove_validator() {
        let mut consensus = PoCConsensus::new(0.66, 0.51).unwrap();
        assert!(consensus.add_validator("validator1".to_string(), 0.8).is_ok());
        assert!(consensus.add_validator("validator2".to_string(), 0.7).is_ok());
        assert!(consensus.remove_validator("validator1").is_ok());
        assert_eq!(consensus.validators.len(), 1);
    }

    #[test]
    fn test_process_new_block() {
        let mut consensus = PoCConsensus::new(0.66, 0.51).unwrap();
        consensus.add_validator("validator1".to_string(), 0.8).unwrap();
        consensus.add_validator("validator2".to_string(), 0.7).unwrap();

        let genesis_block = create_test_block(0, "0");
        consensus.add_block_to_chain(genesis_block).unwrap();

        let new_block = create_test_block(1, "test_hash_0");
        assert!(consensus.process_new_block(new_block).is_ok());

        let blockchain = consensus.get_blockchain().unwrap();
        assert_eq!(blockchain.len(), 2);
    }

    #[test]
    fn test_update_validator_reputation() {
        let mut consensus = PoCConsensus::new(0.66, 0.51).unwrap();
        consensus.add_validator("validator1".to_string(), 0.8).unwrap();
        assert!(consensus.update_validator_reputation("validator1", 0.9).is_ok());
        assert_eq!(consensus.validators["validator1"].reputation, 0.9);
    }
}
