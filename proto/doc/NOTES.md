- paxos
- cascade theorem


# https://datatracker.ietf.org/doc/draft-mazieres-dinrg-scp/
- https://github.com/stanford-scs/scp-spec



Each node choses one or more quorum slices, which are sets of nodes that all include the node itself.
Every message in the SCP protocol specifies the sender's quorum slices.

every protocol message is paired with a cryptographic hash of the sender's "SCPSlices" and digitally signed.


# https://8130068.fs1.hubspotusercontent-na1.net/hubfs/8130068/stellar-consensus-protocol.pdf

phases:
- prepare:
  - fv over "prepare v"
    -  a node votes, accepts, or confirms that  is prepared iff it votes for, accepts, or confirms, respectively, all of these aborts.
-  confirm:
   -  fv over "commit v"
-  externalize:

state
- ϕ = current phase 
- b = current ballot
- p, p' = the two most recent incompatible ballots it has prepared
- c, h
  - in prepare:
    - h is the highest ballot confirmed as prepared, or 0 if none
    - if c != 0, lowest and highest ballots for which v has voted commit and not accepted abort
  - in confirm:
    - lowest and highest ballot for which v accepted commit
  - in externalize:
    - lowest, highest ballot for which v confirmed commit
- z = a next value to try if the current ballot fails
  - if h = 0, then z from nomination
  - otherwise, z = h.x
- M = the latest message received from each node

invariants
- ballots b, p, p', and h are non-decreasing within a phase
- if c != 0, then 
  - c <= h <= b
  - c.x = h.x = b.x

msgs
- PREPARE v i b p p' c.n h.n Q(v)
  - `vote-or-accept(abort b')` for `b' <!= b` a vote to prepare b
  - `accept(abort b')` for `b' <!= p` a vote to confirm p is prepared
  - `accept(abort b')` for `b' <!= p'` a vote to confirm p' is prepared
  - `commit(b')` for `c.n != 0` and `c <= b' <= h` a vote to commit c to h, if c != 0
- CONFIRM v i b p.n c.n h.n Q(v)
  - implies p.x = c.x = h.x = b.x at v
  - p' = 0, because p` is irrelevant after commit
  - implies PREPARE v i <∞, b.x> p 0 c.n ∞ Q(v)
  - `accept(commit b')` for `c <= b' <= h` a vote to confirm commit (c to h)

prepare/confirm/commit

logic
- when h = 0, a node may update z in response to a NOMINATE message
- "a || accept(a)" is what each node must assert for a quorum to accept a

init 
- ϕ ← prepare
- z ← ()
- b ← <0, z>
- M ← {}
- p, p', c, h ← <0, ()>

program
- once z != (), if b.n = 0 then b ← <1, z>
- nodes then repeatedly exchange messages with peers, sending whichever ballot message is indicated by ϕ
- newly received message m at v, added to M_v
  - (1) 