// clang-format off
%#include "scp.hh"

namespace scp {
// clang-format on

program SCP_Control
{
  version SCP_Control1
  {
    void scp_control_null(void) = 0;

    // arguments: slot index, value to nominate, previous value
    bool scp_control_nominate(SCPSlotIndex, SCPValue, SCPValue) = 10;
    void scp_control_stop_nomination(SCPSlotIndex) = 20;

    void scp_control_update_local_quorum_set(SCPQuorumSet) = 100;

    void scp_control_received_envelope(SCPEnvelope) = 200;
    void scp_control_set_state_from_envelope(SCPSlotIndex, SCPEnvelope) = 201;

    SCPOptQuorumSet scp_control_set_qset(SCPQuorumSet) = 300;
    SCPOptQuorumSet scp_control_get_qset(SCPHash) = 301;
    SCPOptQuorumSet scp_control_remove_qset(SCPHash) = 302;

    void scp_control_set_lowest_non_externalized_slot(SCPSlotIndex) = 400;
  }
  = 1;
}
= 0x9aa93003;

} // namespace scp
