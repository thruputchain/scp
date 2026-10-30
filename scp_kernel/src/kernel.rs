use crate::*;
use tracing::debug;

pub struct StellarKernel<C: Consensus, D: Driver> {
    cd: ConsensusDriver<C, D>,
    state: StellarState,
}

impl<C: Consensus, D: Driver> StellarKernel<C, D> {
    pub fn new(
        node_id: thruput_scp::SCPNodeID,
        is_validator: bool,
        qset: thruput_scp::SCPQuorumSet,
        consensus: C,
        driver: D,
    ) -> StellarKernel<C, D> {
        debug!(
            "initializing SCP with local node id {} and qset {}",
            driver.to_short_string(&node_id),
            serde_json::to_string_pretty(&qset).unwrap()
        );
        let driver_clone = driver.clone();
        StellarKernel {
            cd: ConsensusDriver { consensus, driver },
            state: StellarState::new(
                node_id,
                is_validator,
                qset,
                |opaque: &[u8]| driver_clone.get_hash_of(opaque),
            ),
        }
    }

    pub fn proto(&mut self) -> StellarProto<'_, C, D> {
        StellarProto {
            cd: &self.cd,
            state: &mut self.state,
        }
    }
}
