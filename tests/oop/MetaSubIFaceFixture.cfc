/**
 * Sub-interface that extends another interface — its metadata must expose a
 * non-empty `extends` struct (Lucee/ACF key interface `extends` by parent FQN).
 */
interface extends="oop.MetaIFaceFixture" {
	public string function farewell();
}
