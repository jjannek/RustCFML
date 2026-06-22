<cfcomponent>
	<cfscript>
	this.dataKey = "dval";

	public void function configure() {}
	public string function greet() { return "hi"; }
	private string function secret() { return "shh"; }
	</cfscript>
</cfcomponent>
