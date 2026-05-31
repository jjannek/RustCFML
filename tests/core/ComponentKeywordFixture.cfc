// Regression guard for the `component` soft-keyword fix: a real component
// declaration whose header carries a metadata attribute (`output="false"`).
// The discriminator that decides "declaration vs. identifier" must still treat
// this as a declaration even though `output` lexes as a reserved keyword token
// (not a plain identifier). `displayname` exercises the same path with a plain
// identifier key.
component output="false" displayname="ComponentKeywordFixture" {

	function ping() {
		return "pong";
	}

}
