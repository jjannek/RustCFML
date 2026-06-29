component {
	// Relays the processor closure through a SECOND CFC-method frame, so the
	// closure runs two CFC boundaries below its defining frame.
	public any function go( required any processor ) {
		return new CwbSvc().relay( arguments.processor );
	}
}
