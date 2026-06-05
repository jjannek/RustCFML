/**
 * Regression fixture: a component with BOTH a `property name="config"` accessor
 * AND a same-named method `config()`, where the pseudo-constructor (via reset())
 * assigns `variables.config = {...}`. This is the exact shape of WireBox's
 * coldbox.system.ioc.config.Binder (property `scopeRegistration` + method
 * `scopeRegistration()` + reset() seeding it from DEFAULTS).
 *
 * Lucee/ACF hoist methods into the variables scope FIRST, then run the
 * pseudo-constructor, so the `variables.config = value` write shadows the
 * same-named method and getConfig() reads back the value. The getter must read
 * the variables backing, not `this` (where the public method lives).
 */
component accessors="true" {

	property name="config" type="struct";

	variables.DEFAULTS = { config : { enabled : true, mode : "live" } };
	reset();

	function init(){
		return this;
	}

	function reset(){
		variables.config = variables.DEFAULTS.config;
		return this;
	}

	// same-named method — must remain callable while the property still reads back
	function config( key = "" ){
		structAppend( variables.config, { lastKey : arguments.key }, true );
		return this;
	}

}
