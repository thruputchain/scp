// statement = prepare/abort/commit ballot(n, x) for predicate(n) and predicate(x)
// predicate(n) = "n <= k"
// predicate(x) = "all x" || "x"

// FV:
//  - impl "if exists quorum that accepts s, then accept s"
//      -> track who voted/accepted/confirmed s
//  - impl "if v cannot accept s and t, such that t contradicts s"

pub trait FVContext {
    type Node;
    type Statement;
    type QuorumSet;
}

pub trait FVSend<X: FVContext> {
    fn send_vote(s: &X::Statement);
    fn send_accept(s: &X::Statement);
    fn send_confirm(s: &X::Statement);
}

pub trait FVReceived<X: FVContext> {
    fn received_vote(from: &X::Node, qs: &X::QuorumSet, s: &X::Statement);
    fn received_accept(from: &X::Node, qs: &X::QuorumSet, s: &X::Statement);
    fn received_confirm(from: &X::Node, qs: &X::QuorumSet, s: &X::Statement);
}

pub trait FVStateChanged<X: FVContext> {
    //
    fn voted(s: &X::Statement);

    // - v has never accepted a statement contradicting s
    // - either
    //  - there is a quorum U with v in it, where each member voted-or-accepted s
    //  - there is a v-blocking set, where each member accepted s
    fn accepted(s: &X::Statement);

    // there is a quorum U with v in it, where each member voted-or-accepted s
    fn confirmed(s: &X::Statement);
}
