// Gap C fixture: `interface extends="..."` (attribute form, quoted value). On
// Lucee/Adobe CF/BoxLang this parses; RustCFML used to reject it at the `=`
// ("Expected LBrace, found Equal") because interface only accepted the bareword
// `extends Foo` form. Extends a base interface.
interface extends="IDeclCreature" {
	public string function bark();
}
