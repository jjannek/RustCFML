<!---
	Tag-based CFC reproducing two TestBox 5.4.0 / MockBox parsing bugs (GitHub #177):
	  - a multi-line <cfargument> (attributes spanning lines) must still be bound
	    as the first positional parameter;
	  - a <cffunction> with a dotted component-path returntype must not be
	    silently dropped from the component.
--->
<cfcomponent output="false">

	<cffunction name="init" access="public" returntype="any" output="false">
		<cfreturn this>
	</cffunction>

	<!--- multi-line first <cfargument>, mirroring MockBox.createMock --->
	<cffunction
		name      ="cm"
		output    ="false"
		access    ="public"
		returntype="any"
	>
		<cfargument
			name    ="className"
			type    ="string"
			required="false"
			hint    ="The class name"
		/>
		<cfargument name="object" type="any" required="false"/>
		<cfargument
			name    ="clearMethods"
			type    ="boolean"
			required="false"
			default ="false"
		/>
		<cfscript>
		if ( isNull( arguments.className ) ) {
			return "NULL-CLASSNAME";
		}
		return arguments.className;
		</cfscript>
	</cffunction>

	<!--- dotted component-path returntype, mirroring MockBox.getMockGenerator --->
	<cffunction
		name      ="getHelper"
		access    ="public"
		returntype="MockArgHelper"
		output    ="false"
	>
		<cfreturn new MockArgHelper()>
	</cffunction>

</cfcomponent>
