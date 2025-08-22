use crate::*;

pub struct StellarKernel<C: Consensus, D: Driver> {
    cd: ConsensusDriver<C, D>,
    state: StellarState,
}

impl<C: Consensus, D: Driver> StellarKernel<C, D> {
    pub fn new(
        node_id: proto::SCPNodeID,
        is_validator: bool,
        qset: proto::SCPQuorumSet,
        consensus: C,
        driver: D,
    ) -> StellarKernel<C, D> {
        let driver_clone = driver.clone();
        StellarKernel {
            cd: ConsensusDriver { consensus, driver },
            state: StellarState::new(node_id, is_validator, qset, |opaque: &[u8]| {
                driver_clone.get_hash_of(opaque)
            }),
        }
    }

    pub fn proto(&mut self) -> StellarProto<'_, C, D> {
        StellarProto {
            cd: &self.cd,
            state: &mut self.state,
        }
    }
}
