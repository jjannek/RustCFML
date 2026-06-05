/**
 * Regression fixture: init() with an OPTIONAL parameter (`name`) sandwiched
 * between required ones — the exact shape of coldbox.system.ioc.Provider.init
 * (required scopeRegistration, optional name, required targetObject/injectorName).
 * `new NamedArgCtor( ... )` must bind named arguments BY NAME, not by call
 * position, even when the call order differs from the declared parameter order.
 */
component accessors="true" {

	property name="name";
	property name="targetObject";

	function init(
		required struct meta,
		name,
		required targetObject,
		required tag
	){
		variables.meta         = arguments.meta;
		if ( !isNull( arguments.name ) ) {
			variables.name = arguments.name;
		}
		variables.targetObject = arguments.targetObject;
		variables.tag          = arguments.tag;
		return this;
	}

	function getTag(){
		return variables.tag;
	}

	function getMeta(){
		return variables.meta;
	}

	function hasName(){
		return !isNull( variables.name );
	}

}
