use crate::*;

pub struct StellarProto<C: Consensus, D: Driver> {
    pub consensus: C,
    pub driver: D,
}

impl<C: Consensus, D: Driver> StellarProto<C, D> {
    pub fn new(consensus: C, driver: D) -> StellarProto<C, D> {
        return StellarProto { consensus, driver };
    }
}
