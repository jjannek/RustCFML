/**
 * Mirrors Preside's core page.cfc: a script-based component whose body uses a
 * `<!--- ... --->` tag-style comment as a section marker, single-line
 * properties with many attributes, a `labelfield` component attribute, and a
 * method building SQL with deeply nested string interpolation (a `#...#`
 * expression containing a nested string that itself contains `#...#`).
 *
 * @feature sitetree
 */
component labelfield="title" displayname="Tag Comment Props" siteFiltered=true useDrafts=true {

<!--- properties --->
	property name="title"        type="string"  dbtype="varchar" maxLength="200" required=true control="textinput";
	property name="slug"         type="string"  dbtype="varchar" maxLength="50"  required=false uniqueindexes="slug|2" format="slug" cloneable=true;
	property name="page_type"    type="string"  dbtype="varchar" maxLength="100" required=true control="pageTypePicker" indexes="pagetype" autofilter=false;

	<!--- relationships --->
	property name="parent_page"  relationship="many-to-one" relatedTo="page" required=false ondelete="cascade-if-no-cycle-check" onupdate="cascade-if-no-cycle-check";
	property name="child_pages"  relationship="one-to-many" relatedTo="page" relationshipKey="parent_page";

	public string function buildSql( required string col ) output=false {
		var sql = "";
		// Nested interpolation: outer string -> #concat(...)# -> nested string
		// 'Right( ..., #len( 'x' )# - ? )' which itself interpolates.
		sql &= ', #concat( '?', 'Right( #arguments.col#, #lenFn( '#arguments.col#' )# - ? )' )#';
		return sql;
	}

	private string function concat( required string a, required string b ) {
		return arguments.a & arguments.b;
	}
	private string function lenFn( required string x ) {
		return "LEN(" & arguments.x & ")";
	}
}
