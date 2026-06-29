<cfscript>
// Test Java shims - System class static methods
// Note: In real Java, System is a final class and cannot be instantiated.
// You call static methods directly on the class without init()

suiteBegin("Java System Static Methods");

// Test currentTimeMillis (static method) - no init() needed on real Java
javaSystem = createObject("java", "java.lang.System");
currentTime = javaSystem.currentTimeMillis();
assertTrue("currentTimeMillis returns numeric", isNumeric(currentTime));
assertTrue("currentTimeMillis is positive", currentTime gt 0);

// Test getProperty (static method)
osName = javaSystem.getProperty("os.name");
assertTrue("getProperty returns string", isSimpleValue(osName));
assertTrue("os.name is not empty", len(osName) gt 0);

// Test getProperty for different keys
fileSep = javaSystem.getProperty("file.separator");
assertTrue("file.separator is / or \\", fileSep eq "/" or fileSep eq "\\");

userDir = javaSystem.getProperty("user.dir");
assertTrue("user.dir returns string", isSimpleValue(userDir));

// Test getEnv (returns a Map-like object)
env = javaSystem.getEnv();
assertTrue("getEnv returns something", isStruct(env) or isObject(env));

// No-arg getEnv() returns a java.util Map shim, so Map member methods must
// dispatch — `getEnv().get(name)` is how Preside's _getEnvironmentVariable
// reads config. Before the fix it returned a plain struct whose .get() did
// not dispatch and silently returned null. PATH is set in every environment.
assert("getEnv().get(PATH) matches single-arg getEnv(PATH)", javaSystem.getEnv().get("PATH"), javaSystem.getEnv("PATH"));
assertTrue("getEnv().containsKey(PATH)", javaSystem.getEnv().containsKey("PATH"));
assertNull("getEnv().get(missing) is null", javaSystem.getEnv().get("RUSTCFML_DEFINITELY_UNSET_VAR_XYZ"));
assertTrue("getEnv().keySet() includes PATH", arrayContains(javaSystem.getEnv().keySet(), "PATH") gt 0);

// Test nanoTime (static method)
nano = javaSystem.nanoTime();
assertTrue("nanoTime returns numeric", isNumeric(nano));
assertTrue("nanoTime is positive", nano gt 0);

// Test System.out (static field) - on real Java, access via createObject without init
out = javaSystem.out;
assertTrue("System.out exists", isObject(out));

// identityHashCode (GitHub #209) - must return a non-null int; same reference
// yields the same hash, distinct objects yield distinct hashes. CacheBox's
// CacheFactory and TestBox's assertSame/assertNotSame depend on this.
a = { x : 1 };
b = { x : 1 };
ha = javaSystem.identityHashCode( a );
assertFalse("identityHashCode not null", isNull( ha ));
assertTrue("identityHashCode returns numeric", isNumeric( ha ));
assertTrue("identityHashCode stable for same ref", ha eq javaSystem.identityHashCode( a ));
assertTrue("identityHashCode distinct for distinct objects", ha neq javaSystem.identityHashCode( b ));
// Aliases share identity (reference semantics).
c = a;
assertTrue("alias shares identity hash", javaSystem.identityHashCode( c ) eq ha);
// Components are objects too.
greeter = createObject("component", "oop.Greeter");
assertTrue("component identityHashCode numeric", isNumeric( javaSystem.identityHashCode( greeter ) ));

suiteEnd();
</cfscript>