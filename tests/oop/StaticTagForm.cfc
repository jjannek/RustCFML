<cfcomponent>

	<cfstatic>
		<cfset static.TAG_VAL = "from-cfstatic">
		<cfset plain = 7>
	</cfstatic>

	<cffunction name="init"><cfreturn this></cffunction>

	<cffunction name="scoped"><cfreturn static.TAG_VAL></cffunction>

	<cffunction name="plainVal"><cfreturn static.plain></cffunction>

</cfcomponent>
