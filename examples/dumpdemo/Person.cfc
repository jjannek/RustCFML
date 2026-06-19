component {
	property name="firstName";
	property name="age";

	this.firstName = "Ada";
	this.age       = 36;
	this.tags      = [ "math", "code" ];
	this.address   = { city: "London", postcode: "EC1" };

	public string function greet( required string who ) {
		return "hi " & who;
	}
}
