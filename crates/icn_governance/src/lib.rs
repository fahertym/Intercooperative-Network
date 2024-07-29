// File: crates/icn_governance/src/lib.rs

use chrono::{DateTime, Utc, Duration};
use serde::{Serialize, Deserialize};
use icn_common::{IcnResult, IcnError};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalCategory {
    Constitutional,
    Economic,
    Technical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Executed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalType {
    Constitutional,
    EconomicAdjustment,
    NetworkUpgrade,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub proposer: String,
    pub created_at: DateTime<Utc>,
    pub voting_ends_at: DateTime<Utc>,
    pub status: ProposalStatus,
    pub proposal_type: ProposalType,
    pub category: ProposalCategory,
    pub required_quorum: f64,
    pub execution_timestamp: Option<DateTime<Utc>>,
}

impl Proposal {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        title: String,
        description: String,
        proposer: String,
        voting_period: Duration,
        proposal_type: ProposalType,
        category: ProposalCategory,
        required_quorum: f64,
        execution_timestamp: Option<DateTime<Utc>>,
    ) -> Self {
        let now = Utc::now();
        Proposal {
            id,
            title,
            description,
            proposer,
            created_at: now,
            voting_ends_at: now + voting_period,
            status: ProposalStatus::Active,
            proposal_type,
            category,
            required_quorum,
            execution_timestamp,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub voter: String,
    pub proposal_id: String,
    pub in_favor: bool,
    pub weight: f64,
    pub timestamp: DateTime<Utc>,
}

impl Vote {
    pub fn new(voter: String, proposal_id: String, in_favor: bool, weight: f64) -> Self {
        Vote {
            voter,
            proposal_id,
            in_favor,
            weight,
            timestamp: Utc::now(),
        }
    }
}

pub struct GovernanceSystem {
    proposals: HashMap<String, Proposal>,
    votes: HashMap<String, Vec<Vote>>,
}

impl GovernanceSystem {
    pub fn new() -> Self {
        GovernanceSystem {
            proposals: HashMap::new(),
            votes: HashMap::new(),
        }
    }

    pub fn create_proposal(&mut self, proposal: Proposal) -> IcnResult<String> {
        if self.proposals.contains_key(&proposal.id) {
            return Err(IcnError::Governance("Proposal ID already exists".into()));
        }
        let proposal_id = proposal.id.clone();
        self.proposals.insert(proposal_id.clone(), proposal);
        self.votes.insert(proposal_id.clone(), Vec::new());
        Ok(proposal_id)
    }

    pub fn get_proposal(&self, proposal_id: &str) -> Option<&Proposal> {
        self.proposals.get(proposal_id)
    }

    pub fn vote_on_proposal(&mut self, proposal_id: &str, voter: String, in_favor: bool, weight: f64) -> IcnResult<()> {
        let proposal = self.proposals.get_mut(proposal_id)
            .ok_or_else(|| IcnError::Governance("Proposal not found".into()))?;

        if proposal.status != ProposalStatus::Active {
            return Err(IcnError::Governance("Proposal is not active".into()));
        }

        if Utc::now() > proposal.voting_ends_at {
            return Err(IcnError::Governance("Voting period has ended".into()));
        }

        let votes = self.votes.get_mut(proposal_id)
            .ok_or_else(|| IcnError::Governance("Votes not found for proposal".into()))?;

        if votes.iter().any(|v| v.voter == voter) {
            return Err(IcnError::Governance("Voter has already voted on this proposal".into()));
        }

        votes.push(Vote::new(voter, proposal_id.to_string(), in_favor, weight));
        Ok(())
    }

    pub fn finalize_proposal(&mut self, proposal_id: &str) -> IcnResult<ProposalStatus> {
        let proposal = self.proposals.get_mut(proposal_id)
            .ok_or_else(|| IcnError::Governance("Proposal not found".into()))?;

        if proposal.status != ProposalStatus::Active {
            return Err(IcnError::Governance("Proposal is not active".into()));
        }

        if Utc::now() < proposal.voting_ends_at {
            return Err(IcnError::Governance("Voting period has not ended yet".into()));
        }

        let votes = self.votes.get(proposal_id)
            .ok_or_else(|| IcnError::Governance("Votes not found for proposal".into()))?;

        let total_votes: f64 = votes.iter().map(|v| v.weight).sum();
        let votes_in_favor: f64 = votes.iter().filter(|v| v.in_favor).map(|v| v.weight).sum();

        if total_votes < proposal.required_quorum {
            proposal.status = ProposalStatus::Rejected;
        } else if votes_in_favor / total_votes > 0.5 {
            proposal.status = ProposalStatus::Passed;
        } else {
            proposal.status = ProposalStatus::Rejected;
        }

        Ok(proposal.status.clone())
    }

    pub fn list_active_proposals(&self) -> Vec<&Proposal> {
        self.proposals.values()
            .filter(|p| p.status == ProposalStatus::Active)
            .collect()
    }

    pub fn mark_as_executed(&mut self, proposal_id: &str) -> IcnResult<()> {
        let proposal = self.proposals.get_mut(proposal_id)
            .ok_or_else(|| IcnError::Governance("Proposal not found".into()))?;
        
        if proposal.status != ProposalStatus::Passed {
            return Err(IcnError::Governance("Proposal has not passed".into()));
        }

        proposal.status = ProposalStatus::Executed;
        Ok(())
    }
}

impl Default for GovernanceSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_proposal(id: &str) -> Proposal {
        Proposal::new(
            id.to_string(),
            "Test Proposal".to_string(),
            "This is a test proposal".to_string(),
            "Alice".to_string(),
            Duration::days(7),
            ProposalType::Constitutional,
            ProposalCategory::Technical,
            0.5,
            None,
        )
    }

    #[test]
    fn test_create_proposal() {
        let mut gov_system = GovernanceSystem::new();
        let proposal = create_test_proposal("prop1");
        let proposal_id = gov_system.create_proposal(proposal).unwrap();
        assert_eq!(proposal_id, "prop1");
        assert!(gov_system.get_proposal("prop1").is_some());
    }

    #[test]
    fn test_vote_on_proposal() {
        let mut gov_system = GovernanceSystem::new();
        let proposal = create_test_proposal("prop1");
        gov_system.create_proposal(proposal).unwrap();

        assert!(gov_system.vote_on_proposal("prop1", "Alice".to_string(), true, 1.0).is_ok());
        assert!(gov_system.vote_on_proposal("prop1", "Bob".to_string(), false, 1.0).is_ok());

        // Test duplicate vote
        assert!(gov_system.vote_on_proposal("prop1", "Alice".to_string(), false, 1.0).is_err());

        // Test vote on non-existent proposal
        assert!(gov_system.vote_on_proposal("prop2", "Charlie".to_string(), true, 1.0).is_err());
    }

    #[test]
    fn test_finalize_proposal() {
        let mut gov_system = GovernanceSystem::new();
        let mut proposal = create_test_proposal("prop1");
        proposal.voting_ends_at = Utc::now() - Duration::hours(1); // Set voting period to have ended
        gov_system.create_proposal(proposal).unwrap();

        gov_system.vote_on_proposal("prop1", "Alice".to_string(), true, 1.0).unwrap();
        gov_system.vote_on_proposal("prop1", "Bob".to_string(), true, 1.0).unwrap();
        gov_system.vote_on_proposal("prop1", "Charlie".to_string(), false, 1.0).unwrap();

        let result = gov_system.finalize_proposal("prop1").unwrap();
        assert_eq!(result, ProposalStatus::Passed);

        // Test finalizing an already finalized proposal
        assert!(gov_system.finalize_proposal("prop1").is_err());
    }

    #[test]
    fn test_list_active_proposals() {
        let mut gov_system = GovernanceSystem::new();
        let proposal1 = create_test_proposal("prop1");
        let proposal2 = create_test_proposal("prop2");
        let mut proposal3 = create_test_proposal("prop3");
        proposal3.status = ProposalStatus::Passed;

        gov_system.create_proposal(proposal1).unwrap();
        gov_system.create_proposal(proposal2).unwrap();
        gov_system.create_proposal(proposal3).unwrap();

        let active_proposals = gov_system.list_active_proposals();
        assert_eq!(active_proposals.len(), 2);
        assert!(active_proposals.iter().any(|p| p.id == "prop1"));
        assert!(active_proposals.iter().any(|p| p.id == "prop2"));
    }

    #[test]
    fn test_mark_as_executed() {
        let mut gov_system = GovernanceSystem::new();
        let mut proposal = create_test_proposal("prop1");
        proposal.status = ProposalStatus::Passed;
        gov_system.create_proposal(proposal).unwrap();

        assert!(gov_system.mark_as_executed("prop1").is_ok());
        let executed_proposal = gov_system.get_proposal("prop1").unwrap();
        assert_eq!(executed_proposal.status, ProposalStatus::Executed);

        // Test marking a non-passed proposal as executed
        let proposal2 = create_test_proposal("prop2");
        gov_system.create_proposal(proposal2).unwrap();
        assert!(gov_system.mark_as_executed("prop2").is_err());
    }

    #[test]
    fn test_quorum_requirement() {
        let mut gov_system = GovernanceSystem::new();
        let mut proposal = create_test_proposal("prop1");
        proposal.required_quorum = 3.0;
        proposal.voting_ends_at = Utc::now() - Duration::hours(1);
        gov_system.create_proposal(proposal).unwrap();

        gov_system.vote_on_proposal("prop1", "Alice".to_string(), true, 1.0).unwrap();
        gov_system.vote_on_proposal("prop1", "Bob".to_string(), true, 1.0).unwrap();

        let result = gov_system.finalize_proposal("prop1").unwrap();
        assert_eq!(result, ProposalStatus::Rejected); // Rejected due to not meeting quorum

        // Now test with meeting quorum
        let mut proposal2 = create_test_proposal("prop2");
        proposal2.required_quorum = 3.0;
        proposal2.voting_ends_at = Utc::now() - Duration::hours(1);
        gov_system.create_proposal(proposal2).unwrap();

        gov_system.vote_on_proposal("prop2", "Alice".to_string(), true, 1.5).unwrap();
        gov_system.vote_on_proposal("prop2", "Bob".to_string(), true, 1.5).unwrap();

        let result2 = gov_system.finalize_proposal("prop2").unwrap();
        assert_eq!(result2, ProposalStatus::Passed); // Passed due to meeting quorum and majority
    }
}