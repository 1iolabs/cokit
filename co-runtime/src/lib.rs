pub struct CoState {
	// consensus: Consensus,
	/// Settings.
	setting: BTreeMap<String, Ipld>,

	/// Participants of this CO.
	participants: BTreeSet<DID>,

	/// Participant devices of this CO.
	/// These peers may be used to fetch data.
	/// TODO: Do we have very large peer lists?
	known_peers: BTreeSet<PeerId>,
}
