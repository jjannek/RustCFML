<cfscript>
suiteBegin("java.net.InetAddress loopback detection");

// java.net.InetAddress.isLoopbackAddress() must recognise the IPv4 127.0.0.0/8
// block and the IPv6 loopback ::1 in any zero-padded / compressed form. The
// shim previously had no isLoopbackAddress() arm (returned null) and getByName()
// hardcoded the address to 127.0.0.1 for every input — breaking Wheels'
// ConsoleEvalSecuritySpec loopback gating.

function loopback(ip) {
	return CreateObject("java", "java.net.InetAddress").getByName(ip).isLoopbackAddress();
}

assertTrue("127.0.0.1 is loopback", loopback("127.0.0.1"));
assertTrue("127.0.0.53 is loopback (whole /8)", loopback("127.0.0.53"));
assertTrue("::1 compressed IPv6 is loopback", loopback("::1"));
assertTrue("0:0:0:0:0:0:0:1 full IPv6 is loopback", loopback("0:0:0:0:0:0:0:1"));
assertTrue("zero-padded IPv6 is loopback", loopback("0000:0000:0000:0000:0000:0000:0000:0001"));

assertFalse("8.8.8.8 is not loopback", loopback("8.8.8.8"));
assertFalse("10.0.0.1 is not loopback", loopback("10.0.0.1"));
assertFalse("fe80::1 link-local is not loopback", loopback("fe80::1"));

// getHostAddress() round-trips the supplied address (no longer hardcoded).
assert("getHostAddress round-trips", CreateObject("java", "java.net.InetAddress").getByName("8.8.8.8").getHostAddress(), "8.8.8.8");

suiteEnd();
</cfscript>
