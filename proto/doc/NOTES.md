- https://github.com/stanford-scs/scp-spec
- https://datatracker.ietf.org/doc/draft-mazieres-dinrg-scp/



Each node choses one or more quorum slices, which are sets of nodes that all include the node itself.
Every message in the SCP protocol specifies the sender's quorum slices.

every protocol message is paired with a cryptographic hash of the sender's "SCPSlices" and digitally signed.
