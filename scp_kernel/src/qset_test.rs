#[cfg(test)]
use crate::qset::*;
use sha2::{Digest, Sha256};
use thruput_scp::*;
use thruput_xdr::types::{Ed25519PublicKey, PublicKey};
use xdr_common::VecM;

// Helper to create VecM from Vec easily
fn vecm<T>(vec: Vec<T>) -> VecM<T> {
    VecM(vec)
}

// Helper function to create public keys like the C++ version
fn make_public_key(i: i32) -> PublicKey {
    let seed_string = format!("NODE_SEED_{}", i);
    let hash = Sha256::digest(seed_string.as_bytes());
    let hash_bytes: [u8; 32] = hash.into();
    let ed25519_key = Ed25519PublicKey::from(hash_bytes);
    PublicKey::Ed25519 {
        ed25519: ed25519_key,
    }
}

// Helper function to create a singleton quorum set
fn make_singleton(key: PublicKey) -> SCPQuorumSet {
    SCPQuorumSet {
        threshold: 1,
        validators: vecm(vec![key]),
        inner_sets: vecm(vec![]),
    }
}

// Helper function to create keys and sort them like the C++ version
fn create_sorted_keys(count: usize) -> Vec<PublicKey> {
    let mut keys = Vec::new();
    for i in 0..count {
        keys.push(make_public_key(i as i32));
    }
    keys.sort_by(crate::qset::compare_public_key);
    keys
}

// Helper function that mimics the C++ check function
fn check_qset(
    qset_check: &SCPQuorumSet,
    expected_sanity: bool,
    expected_normalized: &SCPQuorumSet,
    expected_normalized_sanity: bool,
) {
    // First, without normalization
    let sanity_result = is_qset_sane(qset_check, false);
    assert_eq!(expected_sanity, sanity_result.is_ok());

    // Secondary test: normalize the set and check again
    let mut normalized_qset = qset_check.clone();
    normalize_qset(&mut normalized_qset, None);
    let normalized_sanity_result = is_qset_sane(&normalized_qset, false);

    assert_eq!(expected_normalized_sanity, normalized_sanity_result.is_ok());
    assert_eq!(expected_normalized, &normalized_qset);
}

#[test]
fn test_sane_quorum_set() {
    let keys = create_sorted_keys(1001);

    // Test: { t: 0 }
    let qset_t0 = SCPQuorumSet {
        threshold: 0,
        validators: vecm(vec![]),
        inner_sets: vecm(vec![]),
    };
    check_qset(&qset_t0, false, &qset_t0, false);

    let valid_one_node = make_singleton(keys[0].clone());

    // Test: { t: 0, v0 }
    let mut qset_t0_v0 = valid_one_node.clone();
    qset_t0_v0.threshold = 0;
    check_qset(&qset_t0_v0, false, &qset_t0_v0, false);

    // Test: { t: 2, v0 }
    let mut qset_t2_v0 = valid_one_node.clone();
    qset_t2_v0.threshold = 2;
    check_qset(&qset_t2_v0, false, &qset_t2_v0, false);

    // Test: { t: 1, v0 }
    check_qset(&valid_one_node, true, &valid_one_node, true);

    // Test: { t: 1, v0, { t: 1, v1 } } -> { t:1, v0, v1 }
    let qset_with_inner = SCPQuorumSet {
        threshold: 1,
        validators: vecm(vec![keys[0].clone()]),
        inner_sets: vecm(vec![SCPQuorumSet {
            threshold: 1,
            validators: vecm(vec![keys[1].clone()]),
            inner_sets: vecm(vec![]),
        }]),
    };

    let expected_merged = SCPQuorumSet {
        threshold: 1,
        validators: vecm(vec![keys[0].clone(), keys[1].clone()]),
        inner_sets: vecm(vec![]),
    };

    check_qset(&qset_with_inner, true, &expected_merged, true);

    // Test: { t: 1, v0, { t: 2, v1 }, { t: 1, v2 } } -> { t:1, v0, v2, { t: 2, v1 } }
    let qset_mixed = SCPQuorumSet {
        threshold: 1,
        validators: vecm(vec![keys[0].clone()]),
        inner_sets: vecm(vec![
            SCPQuorumSet {
                threshold: 2,
                validators: vecm(vec![keys[1].clone()]),
                inner_sets: vecm(vec![]),
            },
            SCPQuorumSet {
                threshold: 1,
                validators: vecm(vec![keys[2].clone()]),
                inner_sets: vecm(vec![]),
            },
        ]),
    };

    let expected_mixed = SCPQuorumSet {
        threshold: 1,
        validators: vecm(vec![keys[0].clone(), keys[2].clone()]),
        inner_sets: vecm(vec![SCPQuorumSet {
            threshold: 2,
            validators: vecm(vec![keys[1].clone()]),
            inner_sets: vecm(vec![]),
        }]),
    };

    check_qset(&qset_mixed, false, &expected_mixed, false);

    // Test: { t: 1, v0, { t: 1, v1 }, { t: 1, v2, v3 } } -> { t:1, v0, v1, { t: 1, v2, v3 } }
    let valid_multiple_nodes = SCPQuorumSet {
        threshold: 1,
        validators: vecm(vec![keys[0].clone()]),
        inner_sets: vecm(vec![
            SCPQuorumSet {
                threshold: 1,
                validators: vecm(vec![keys[1].clone()]),
                inner_sets: vecm(vec![]),
            },
            SCPQuorumSet {
                threshold: 1,
                validators: vecm(vec![keys[2].clone(), keys[3].clone()]),
                inner_sets: vecm(vec![]),
            },
        ]),
    };

    let valid_multiple_nodes_normalized = SCPQuorumSet {
        threshold: 1,
        validators: vecm(vec![
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            keys[3].clone(),
        ]),
        inner_sets: vecm(vec![]),
    };

    check_qset(
        &valid_multiple_nodes,
        true,
        &valid_multiple_nodes_normalized,
        true,
    );

    // Test containing set simplification
    // Test: { t: 1, { t: 1, v0, { t: 1, v1 }, { t: 1, v2, v3 } } } -> { t:1, v0, v1, { t: 1, v2, v3 } }
    let containing_set = SCPQuorumSet {
        threshold: 1,
        validators: vecm(vec![]),
        inner_sets: vecm(vec![valid_multiple_nodes.clone()]),
    };

    check_qset(
        &containing_set,
        true,
        &valid_multiple_nodes_normalized,
        true,
    );
}

#[test]
fn test_nested_quorum_sets() {
    let keys = create_sorted_keys(10);

    // Test deep nesting
    let test_nesting_level =
        |keys: &[PublicKey],
         nesting_level: usize,
         expected_sanity: bool,
         expected_normalized_sanity: bool| {
            let mut qsets = Vec::new();
            for i in 0..=nesting_level {
                qsets.push(make_singleton(keys[i].clone()));
            }

            for i in (0..nesting_level).rev() {
                qsets[i].inner_sets = vecm(vec![qsets[i + 1].clone()]);
            }

            // Create expected normalized version
            let mut expected_normalized = thruput_scp::SCPQuorumSet {
                threshold: 1,
                validators: VecM(vec![]),
                inner_sets: VecM(vec![]),
            };
            for i in 0..=nesting_level {
                expected_normalized.validators.0.push(keys[i].clone());
            }

            check_qset(
                &qsets[0],
                expected_sanity,
                &expected_normalized,
                expected_normalized_sanity,
            );
        };

    // Test maximum nesting level exceeded
    // Test: { t: 1, v0, { t: 1, v1, { .. t: 1, v_{MAXIMUM_QUORUM_NESTING_LEVEL + 1} }..} -> too deep"
    test_nesting_level(
        &keys,
        MAXIMUM_QUORUM_NESTING_LEVEL as usize + 1,
        false,
        true,
    );

    // Test maximum nesting level allowed
    // Test: { t: 1, v0, { t: 1, v1, { .. t: 1, v_MAXIMUM_QUORUM_NESTING_LEVEL }..} "
    test_nesting_level(
        &keys,
        MAXIMUM_QUORUM_NESTING_LEVEL as usize,
        true,
        true,
    );
}

#[test]
fn test_validator_count_limits() {
    let keys = create_sorted_keys(1001);

    // Test: { t: 1, v0..v999 } -> valid (1000 validators)
    let qset_1000 = SCPQuorumSet {
        threshold: 1,
        validators: vecm(keys[0..1000].to_vec()),
        inner_sets: vecm(vec![]),
    };
    check_qset(&qset_1000, true, &qset_1000, true);

    // Test: { t: 1, v0..v1000 } -> too big (1001 validators)
    let qset_1001 = SCPQuorumSet {
        threshold: 1,
        validators: vecm(keys[0..1001].to_vec()),
        inner_sets: vecm(vec![]),
    };
    check_qset(&qset_1001, false, &qset_1001, false);

    // Test: distributed nodes across inner sets -> too big
    // Test: { t: 2, v0, { t: 1, v1..v100 }, { t: 1, v101..v200} ... { t: 1, v901..v1000} -> too big
    let mut inner_sets = Vec::new();
    for i in 0..10 {
        let inner_set = SCPQuorumSet {
            threshold: 1,
            validators: vecm(keys[(i * 100 + 1)..=((i + 1) * 100)].to_vec()),
            inner_sets: vecm(vec![]),
        };
        inner_sets.push(inner_set);
    }

    let qset_distributed = SCPQuorumSet {
        threshold: 2,
        validators: VecM(vec![keys[0].clone()]),
        inner_sets: VecM(inner_sets),
    };

    check_qset(&qset_distributed, false, &qset_distributed, false);
}

#[test]
fn test_complex_nested_normalization() {
    let keys = create_sorted_keys(4);

    // Test: { t: 1, v0, { t: 1, v1, { t: 1, v2, { t: 1, v3 } } } }
    // Should normalize to: { t: 1, v0, { t: 1, v1, { t: 1, v2, v3 } } }
    let qset = SCPQuorumSet {
        threshold: 1,
        validators: vecm(vec![keys[0].clone()]),
        inner_sets: vecm(vec![SCPQuorumSet {
            threshold: 1,
            validators: vecm(vec![keys[1].clone()]),
            inner_sets: vecm(vec![SCPQuorumSet {
                threshold: 1,
                validators: vecm(vec![keys[2].clone()]),
                inner_sets: vecm(vec![make_singleton(keys[3].clone())]),
            }]),
        }]),
    };

    let expected_normalized = SCPQuorumSet {
        threshold: 1,
        validators: vecm(vec![
            keys[0].clone(),
            keys[1].clone(),
            keys[2].clone(),
            keys[3].clone(),
        ]),
        inner_sets: vecm(vec![]),
    };

    check_qset(&qset, true, &expected_normalized, true);
}

#[test]
fn compare_public_key_orders_ed25519_keys_by_bytes() {
    let key = |bytes: [u8; 32]| PublicKey::Ed25519 {
        ed25519: Ed25519PublicKey::from(bytes),
    };
    let mut low = [0xffu8; 32];
    low[0] = 0x00;
    let mut high = [0x00u8; 32];
    high[0] = 0x01;
    assert_eq!(
        compare_public_key(&key(low), &key(high)),
        std::cmp::Ordering::Less
    );
    assert_eq!(
        compare_public_key(&key(high), &key(high)),
        std::cmp::Ordering::Equal
    );
}

#[test]
fn compare_public_key_orders_other_schemes() {
    let ed25519 = make_public_key(0);
    let mldsa65 = PublicKey::MlDsa65 {
        mldsa65: Box::new([0u8; 1952].into()),
    };
    assert_eq!(
        compare_public_key(&ed25519, &mldsa65),
        std::cmp::Ordering::Less
    );
    assert_eq!(
        compare_public_key(&mldsa65, &ed25519),
        std::cmp::Ordering::Greater
    );
}
