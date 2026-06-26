/**
 * Implements a sibling interface by unqualified name; loaded via a package path
 * (oop.ifacepkg.SiblingMock) it must resolve "SiblingIFace" relative to its own
 * directory, like extends does (issue #206).
 */
component implements="SiblingIFace" {
	function foo(required name){ return "ok:" & arguments.name; }
}
