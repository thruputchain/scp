pub trait Driver {
    /// Retrieves a quorum set from its hash
    ///
    /// All SCP statement (see `SCPNomination` and `SCPStatement`) include
    /// a quorum set hash.
    /// SCP does not define how quorum sets are exchanged between nodes,
    /// hence their retrieval is delegated to the user of SCP.
    /// The return value is not cached by SCP, as quorum sets are transient.
    ///
    /// Nonde is a valid return value which cause the statement to be
    /// considered invalid.
    fn get_qset(&self, h: &proto::SCPHash) -> Option<proto::SCPQuorumSet>;
}
