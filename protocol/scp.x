// Copyright 2015 Stellar Development Foundation and contributors. Licensed
// under the Apache License, Version 2.0. See the COPYING file at the root
// of this distribution or at http://www.apache.org/licenses/LICENSE-2.0

// clang-format off
%#include "types.hh"

namespace scp {
// clang-format on

typedef opaque SCPHash[32];
typedef SCPPublicKey SCPNodeID;

typedef unsigned int SCPCount;
typedef unsigned hyper SCPSlotIndex;

typedef opaque SCPValue<>;
typedef SCPValue SCPValues<>;
typedef SCPValue *SCPOptValue;

struct SCPBallot {
  SCPCount counter; // n
  SCPValue value;   // x
};

enum SCPStatementType {
  SCP_ST_PREPARE = 0,
  SCP_ST_CONFIRM = 1,
  SCP_ST_EXTERNALIZE = 2,
  SCP_ST_NOMINATE = 3
};

struct SCPStatementNominate {
  SCPHash quorum_set_hash; // D
  SCPValue votes<>;        // X
  SCPValue accepted<>;     // Y
};

struct SCPStatementPrepare {
  SCPHash quorum_set_hash;   // D
  SCPBallot ballot;          // b
  SCPBallot *prepared;       // p
  SCPBallot *prepared_prime; // p'
  SCPCount n_c;              // c.n
  SCPCount n_h;              // h.n
};

struct SCPStatementConfirm {
  SCPBallot ballot;        // b
  SCPCount n_prepared;     // p.n
  SCPCount n_commit;       // c.n
  SCPCount n_h;            // h.n
  SCPHash quorum_set_hash; // D
};

struct SCPStatementExternalize {
  SCPBallot commit;               // c
  SCPCount n_h;                   // h.n
  SCPHash commit_quorum_set_hash; // D used before EXTERNALIZE
};

struct SCPStatement {
  SCPNodeID node_id;       // v
  SCPSlotIndex slot_index; // i

  union switch (SCPStatementType stmt_type)
  {
  case SCP_ST_PREPARE:
    SCPStatementPrepare prepare;
  case SCP_ST_CONFIRM:
    SCPStatementConfirm confirm;
  case SCP_ST_EXTERNALIZE:
    SCPStatementExternalize externalize;
  case SCP_ST_NOMINATE:
    SCPStatementNominate nominate;
  }
  pledges;
};

struct SCPEnvelope {
  SCPStatement statement;
  SCPSignature signature;
};

typedef SCPEnvelope SCPEnvelopes<>;

// supports things like: A,B,C,(D,E,F),(G,H,(I,J,K,L))
// only allows 2 levels of nesting
struct SCPQuorumSet {
  SCPCount threshold;
  SCPNodeID validators<>;
  SCPQuorumSet inner_sets<>;
};

typedef SCPQuorumSet *SCPOptQuorumSet;

} // namespace scp
