// Gap C fixture: `extends` AFTER another header attribute on an interface — the
// order-independent attribute rule applies to interfaces too.
interface displayname="pup" extends="IDeclCreature" {
	public string function bark();
}
