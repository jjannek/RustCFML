<cfcomponent output="false">
	<!--- pseudo-constructor state read back through `this` inside the method --->
	<cfset this.parentMarker = "parent-this" />

	<!--- Capital-O name. The child's super call uses the lowercase spelling, so this
	      exercises case-insensitive super dispatch. The method reads `this.parentMarker`;
	      if super dispatch fails to bind the child as `this`, this read throws
	      "Variable 'this' is undefined". --->
	<cffunction name="OnApplicationStart" returntype="string" output="false">
		<cfreturn this.parentMarker />
	</cffunction>
</cfcomponent>
