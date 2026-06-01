// Gap A fixture: `extends` appears AFTER another attribute (`output="false"`).
// On Lucee/Adobe CF/BoxLang component attributes are order-independent, so this
// parses and instantiates exactly like ExtendsFirstFixture.
component output="false" extends="DeclAttrBase" {
	public string function ping() {
		return "pong";
	}
}
