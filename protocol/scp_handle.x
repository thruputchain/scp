// clang-format off
%#include "scp.hh"

namespace hogd {
// clang-format on

enum SCPValidationLevel {
  SCP_INVALID = 0,
  SCP_MAYBE_VALID = 1,
  SCP_FULLY_VALID = 2,
};

program SCP_Handle
{
  version SCP_Handle1
  {
    void scp_handle_null(void) = 0;

    // hooks

    void scp_handle_emit_envelope(SCPEnvelope) = 10;
    SCPValidationLevel scp_handle_validate_ballot_value(SCPSlotIndex,
							SCPValue) = 20;
    SCPValidationLevel scp_handle_validate_nomination_value(SCPSlotIndex,
							    SCPValue) = 30;
    SCPOptValue scp_handle_extract_valid_value(SCPSlotIndex, SCPValue) = 40;
    SCPOptValue scp_handle_combine_candidates(SCPSlotIndex, SCPValues) = 50;

    // notifications

    void scp_handle_value_externalized(SCPSlotIndex, SCPValue) = 100;
    void scp_handle_nominating_value(SCPSlotIndex, SCPValue) = 200;
    void scp_handle_updated_candidate_value(SCPSlotIndex, SCPValue) = 210;
    void scp_handle_started_ballot_proto(SCPSlotIndex, SCPBallot) = 310;
    void scp_handle_accepted_ballot_prepared(SCPSlotIndex, SCPBallot) = 320;
    void scp_handle_confirmed_ballot_prepared(SCPSlotIndex, SCPBallot) = 330;
    void scp_handle_accepted_commit(SCPSlotIndex, SCPBallot) = 340;
    void scp_handle_ballot_did_hear_from_quorum(SCPSlotIndex, SCPBallot) =
    350;

    // utils
    SCPOptQuorumSet scp_handle_get_quorum_set(SCPHash) = 1000;
  }
  = 1;
}
= 0x9aa92002;

} // namespace hogd
