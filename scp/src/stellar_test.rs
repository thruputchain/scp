#[cfg(test)]
use crate::testutil::*;
use proto::*;
use sha2::{Digest, Sha256};
use xdr_common::VecM;

#[test]
fn test_vblocking_and_quorum() {
    let values = TestValues::default();
    let nodes = TestNode::new_batch(4);
}
