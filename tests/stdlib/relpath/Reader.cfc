/**
 * Fixture for GitHub #171: a relative file path passed to a file BIF must
 * resolve against THIS component's directory (matching ExpandPath), not the
 * entry template / process cwd. `reldata.json` sits next to this CFC.
 */
component {

	public function init(){
		return this;
	}

	// dot-relative
	public string function readDotRelative(){
		return fileRead( "./reldata.json" );
	}

	// bare relative (no ./)
	public string function readBareRelative(){
		return fileRead( "reldata.json" );
	}

	// fileExists must use the same base
	public boolean function existsRelative(){
		return fileExists( "./reldata.json" );
	}

	// the documented workaround must agree with the bare relative result
	public string function readViaExpandPath(){
		return fileRead( expandPath( "./reldata.json" ) );
	}
}
