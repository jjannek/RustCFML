// Helper for test_hof_member_writeback.cfm. Exercises a higher-order struct
// member fn (`some`) called INSIDE a CFC method over an instance-variable
// struct whose values are components — the shape of WireBox's
// `binder.hasAspects()` (`mappings.some( (k,m) => m.isAspect() )`).
component {

	function init(){
		variables.items = {
			a : new HofWritebackItem( false ),
			b : new HofWritebackItem( false ),
			c : new HofWritebackItem( false )
		};
		return this;
	}

	// `.some()` over the instance-var struct, closure invokes a method on each
	// component value. Must return false (none flagged).
	function anyFlagged(){
		return variables.items.some( ( key, item ) => {
			return arguments.item.isFlag();
		} );
	}

	// Local-copy variant (the form that used to mis-dispatch `.some()` to the
	// enclosing component once the receiver had been corrupted).
	function anyFlaggedViaLocal(){
		var s = variables.items;
		return s.some( ( key, item ) => {
			return arguments.item.isFlag();
		} );
	}

	// Used to confirm the instance-var struct was not corrupted by the HOF call
	// (i.e. not replaced by the enclosing component `this`). Case/order-agnostic
	// so it is portable across engines.
	function itemCount(){
		return structCount( variables.items );
	}
	function looksLikeComponent(){
		// A `this`-corrupted struct would carry this component's method names.
		return structKeyExists( variables.items, "anyFlagged" );
	}

}
