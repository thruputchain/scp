use crate::*;

pub struct StellarProto<'a, C: Consensus, D: Driver> {
    pub consensus: &'a C,
    pub driver: &'a D,
}

impl<'a, C: Consensus, D: Driver> StellarProto<'a, C, D> {
    pub fn new(consensus: &'a C, driver: &'a D) -> StellarProto<'a, C, D> {
        return StellarProto { consensus, driver };
    }
}
