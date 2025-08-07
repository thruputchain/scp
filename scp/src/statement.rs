// state:
//  - set of (node, statement, {undecided|voted|accepted|confirmed})
//  - statement = {prepare|abort|commit}, ballot
//  - region of statements = subset{prepare|abort|commit} + range{counter} + all-or-one{value}

// invariant, for every node:
//  - no vote on statement contradicting another vote statement
//  - no accept on statement contradicting another accept statement
//  - no vote on statement contradicting an accepted statement

// triggers:
//  - received (node, {prepare/abort/commit}, ballot)
//  - there is a ballot "b" for which the node confirms "prepare(b)" and accepts "commit(b)", i.e.
//      -> accept "prepare(b)" reaches quorum threshold, and
//      -> either:
//          - vote-or-accept "commit(b)" reaches quorum threshold, or
//          - accept "commit(b)" reaches blocking threshold
