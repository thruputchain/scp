namespace hogd {

struct SCPConfig {
	unsigned hyper max_timeout_ms;
	unsigned hyper initial_ballot_timeout_ms;
	unsigned hyper increment_ballot_timeout_ms;
	unsigned hyper initial_nomination_timeout_ms;
	unsigned hyper increment_nomination_timeout_ms;
	
	SCPQuorumSet quorum_set;
};

} // namespace hogd
