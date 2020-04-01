// Copyright 2017-2020 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! The tests for functionality concerning the "external" origin.

use super::*;

#[test]
fn veto_external_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		assert_ok!(Democracy::external_propose(
			Origin::signed(2),
			set_balance_proposal_hash_and_note(2),
		));
		assert!(<NextExternal<Test>>::exists());

		let h = set_balance_proposal_hash_and_note(2);
		assert_ok!(Democracy::veto_external(Origin::signed(3), h.clone()));
		// cancelled.
		assert!(!<NextExternal<Test>>::exists());
		// fails - same proposal can't be resubmitted.
		assert_noop!(Democracy::external_propose(
			Origin::signed(2),
			set_balance_proposal_hash(2),
		), Error::<Test>::ProposalBlacklisted);

		fast_forward_to(1);
		// fails as we're still in cooloff period.
		assert_noop!(Democracy::external_propose(
			Origin::signed(2),
			set_balance_proposal_hash(2),
		), Error::<Test>::ProposalBlacklisted);

		fast_forward_to(2);
		// works; as we're out of the cooloff period.
		assert_ok!(Democracy::external_propose(
			Origin::signed(2),
			set_balance_proposal_hash_and_note(2),
		));
		assert!(<NextExternal<Test>>::exists());

		// 3 can't veto the same thing twice.
		assert_noop!(
			Democracy::veto_external(Origin::signed(3), h.clone()),
			Error::<Test>::AlreadyVetoed
		);

		// 4 vetoes.
		assert_ok!(Democracy::veto_external(Origin::signed(4), h.clone()));
		// cancelled again.
		assert!(!<NextExternal<Test>>::exists());

		fast_forward_to(3);
		// same proposal fails as we're still in cooloff
		assert_noop!(Democracy::external_propose(
			Origin::signed(2),
			set_balance_proposal_hash(2),
		), Error::<Test>::ProposalBlacklisted);
		// different proposal works fine.
		assert_ok!(Democracy::external_propose(
			Origin::signed(2),
			set_balance_proposal_hash_and_note(3),
		));
	});
}

#[test]
fn external_referendum_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		assert_noop!(
			Democracy::external_propose(
				Origin::signed(1),
				set_balance_proposal_hash(2),
			),
			BadOrigin,
		);
		assert_ok!(Democracy::external_propose(
			Origin::signed(2),
			set_balance_proposal_hash_and_note(2),
		));
		assert_noop!(Democracy::external_propose(
			Origin::signed(2),
			set_balance_proposal_hash(1),
		), Error::<Test>::DuplicateProposal);
		fast_forward_to(2);
		assert_eq!(
			Democracy::referendum_status(0),
			Ok(ReferendumStatus {
				end: 4,
				proposal_hash: set_balance_proposal_hash(2),
				threshold: VoteThreshold::SuperMajorityApprove,
				delay: 2,
				tally: Tally { ayes: 0, nays: 0, turnout: 0 },
			})
		);
	});
}

#[test]
fn external_majority_referendum_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		assert_noop!(
			Democracy::external_propose_majority(
				Origin::signed(1),
				set_balance_proposal_hash(2)
			),
			BadOrigin,
		);
		assert_ok!(Democracy::external_propose_majority(
			Origin::signed(3),
			set_balance_proposal_hash_and_note(2)
		));
		fast_forward_to(2);
		assert_eq!(
			Democracy::referendum_status(0),
			Ok(ReferendumStatus {
				end: 4,
				proposal_hash: set_balance_proposal_hash(2),
				threshold: VoteThreshold::SimpleMajority,
				delay: 2,
				tally: Tally { ayes: 0, nays: 0, turnout: 0 },
			})
		);
	});
}

#[test]
fn external_default_referendum_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		assert_noop!(
			Democracy::external_propose_default(
				Origin::signed(3),
				set_balance_proposal_hash(2)
			),
			BadOrigin,
		);
		assert_ok!(Democracy::external_propose_default(
			Origin::signed(1),
			set_balance_proposal_hash_and_note(2)
		));
		fast_forward_to(2);
		assert_eq!(
			Democracy::referendum_status(0),
			Ok(ReferendumStatus {
				end: 4,
				proposal_hash: set_balance_proposal_hash(2),
				threshold: VoteThreshold::SuperMajorityAgainst,
				delay: 2,
				tally: Tally { ayes: 0, nays: 0, turnout: 0 },
			})
		);
	});
}


#[test]
fn external_and_public_interleaving_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		assert_ok!(Democracy::external_propose(
			Origin::signed(2),
			set_balance_proposal_hash_and_note(1),
		));
		assert_ok!(propose_set_balance_and_note(6, 2, 2));

		fast_forward_to(2);

		// both waiting: external goes first.
		assert_eq!(
			Democracy::referendum_status(0),
			Ok(ReferendumStatus {
				end: 4,
				proposal_hash: set_balance_proposal_hash_and_note(1),
				threshold: VoteThreshold::SuperMajorityApprove,
				delay: 2,
				tally: Tally { ayes: 0, nays: 0, turnout: 0 },
			})
		);
		// replenish external
		assert_ok!(Democracy::external_propose(
				Origin::signed(2),
				set_balance_proposal_hash_and_note(3),
			));

		fast_forward_to(4);

		// both waiting: public goes next.
		assert_eq!(
			Democracy::referendum_status(1),
			Ok(ReferendumStatus {
				end: 6,
				proposal_hash: set_balance_proposal_hash_and_note(2),
				threshold: VoteThreshold::SuperMajorityApprove,
				delay: 2,
				tally: Tally { ayes: 0, nays: 0, turnout: 0 },
			})
		);
		// don't replenish public

		fast_forward_to(6);

		// it's external "turn" again, though since public is empty that doesn't really matter
		assert_eq!(
			Democracy::referendum_status(2),
			Ok(ReferendumStatus {
				end: 8,
				proposal_hash: set_balance_proposal_hash_and_note(3),
				threshold: VoteThreshold::SuperMajorityApprove,
				delay: 2,
				tally: Tally { ayes: 0, nays: 0, turnout: 0 },
			})
		);
		// replenish external
		assert_ok!(Democracy::external_propose(
				Origin::signed(2),
				set_balance_proposal_hash_and_note(5),
			));

		fast_forward_to(8);

		// external goes again because there's no public waiting.
		assert_eq!(
			Democracy::referendum_status(3),
			Ok(ReferendumStatus {
				end: 10,
				proposal_hash: set_balance_proposal_hash_and_note(5),
				threshold: VoteThreshold::SuperMajorityApprove,
				delay: 2,
				tally: Tally { ayes: 0, nays: 0, turnout: 0 },
			})
		);
		// replenish both
		assert_ok!(Democracy::external_propose(
			Origin::signed(2),
			set_balance_proposal_hash_and_note(7),
		));
		assert_ok!(propose_set_balance_and_note(6, 4, 2));

		fast_forward_to(10);

		// public goes now since external went last time.
		assert_eq!(
			Democracy::referendum_status(4),
			Ok(ReferendumStatus {
				end: 12,
				proposal_hash: set_balance_proposal_hash_and_note(4),
				threshold: VoteThreshold::SuperMajorityApprove,
				delay: 2,
				tally: Tally { ayes: 0, nays: 0, turnout: 0 },
			})
		);
		// replenish public again
		assert_ok!(propose_set_balance_and_note(6, 6, 2));
		// cancel external
		let h = set_balance_proposal_hash_and_note(7);
		assert_ok!(Democracy::veto_external(Origin::signed(3), h));

		fast_forward_to(12);

		// public goes again now since there's no external waiting.
		assert_eq!(
			Democracy::referendum_status(5),
			Ok(ReferendumStatus {
				end: 14,
				proposal_hash: set_balance_proposal_hash_and_note(6),
				threshold: VoteThreshold::SuperMajorityApprove,
				delay: 2,
				tally: Tally { ayes: 0, nays: 0, turnout: 0 },
			})
		);
	});
}