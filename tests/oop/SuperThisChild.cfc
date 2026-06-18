<cfcomponent extends="oop.SuperThisParent" output="false">

	<!--- lowercase-o override delegating up via a lowercase super call; the parent
	      method is OnApplicationStart, so the names differ only by case. --->
	<cffunction name="onApplicationStart" returntype="string" output="false">
		<cfset variables.parentResult = super.onApplicationStart() />
		<cfreturn "child-ran" />
	</cffunction>

	<cffunction name="getParentResult" returntype="string" output="false">
		<cfreturn variables.parentResult ?: "missing" />
	</cffunction>
</cfcomponent>
