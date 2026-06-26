component {

	/**
	 * @expectedException InvalidException
	 * @skip false
	 * @labels foo,bar
	 * @order 3
	 */
	function annotated( required string a, numeric b = 2 ) {
		return 1;
	}

	function plain() {
		return 2;
	}

}
