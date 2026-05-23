<cfscript>
suiteBegin("cfconfig — loading");

// Skip on Lucee — this suite verifies RustCFML-specific config wiring.
hasCfconfig = isDefined("server") && isStruct(server) && structKeyExists(server, "cfconfig");
if (!hasCfconfig) {
    suiteEnd();
    return;
}

cfg = server.cfconfig;
assert("cfconfig is a struct", isStruct(cfg), true);

// Values from tests/.cfconfig.json
assert("server.port from fixture", cfg.server.port, 9999);
assert("server.welcomeFiles[2]", cfg.server.welcomeFiles[2], "default.cfm");
assert("server.maxRequestBodySize", cfg.server.maxRequestBodySize, 5242880);
assert("runtime.locale", cfg.runtime.locale, "en-GB");
assert("runtime.timezone", cfg.runtime.timezone, "Europe/London");
assert("runtime.sessionTimeout", cfg.runtime.sessionTimeout, "0,0,15,0");

// Defaults filled in where the fixture is silent
assert("server.host default", cfg.server.host, "127.0.0.1");
assert("runtime.dotNotationUpperCase default", cfg.runtime.dotNotationUpperCase, true);

// Datasource declared
assert("datasources.testds.driver", cfg.datasources.testds.driver, "sqlite");

// Security flags
assert("security.csrfEnabled", cfg.security.csrfEnabled, true);
assert("security.disallowedFunctions has __cfexecute", arrayLen(cfg.security.disallowedFunctions), 1);
assert("disallowedFunctions[1]", cfg.security.disallowedFunctions[1], "cfconfigSecurityProbe");

// Debugging
assert("debugging.enabled", cfg.debugging.enabled, true);
assert("debugging.errorStatusCode", cfg.debugging.errorStatusCode, false);

suiteEnd();
</cfscript>
