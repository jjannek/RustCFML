<cfcomponent>
	<!--- Tag-based constructor whose <cfargument> tags carry custom attributes
	      (notably WireBox's inject=). These MUST survive into
	      getMetadata().functions[].parameters[] or ColdBox/WireBox builds the
	      component with zero constructor arguments (Preside cbi18n boot bug). --->
	<cffunction name="init" returntype="PresideFixTagArgInject">
		<cfargument name="controller" inject="coldbox">
		<cfargument name="features" type="struct" inject="coldbox:setting:features">
		<cfargument name="logger" type="any" required="true" inject="logbox" scope="prototype">
		<cfargument name="plain">
		<cfreturn this>
	</cffunction>
</cfcomponent>
