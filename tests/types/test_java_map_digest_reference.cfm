<cfscript>
suiteBegin("Java map reference semantics + MessageDigest.isEqual");

// java.util.concurrent.ConcurrentHashMap is a REFERENCE type: mutating a nested
// map fetched via get() must persist in the outer map. RustCFML's shim used to
// snapshot-and-return on put/remove/clear, so `outer.get(k).put(...)` mutated a
// throwaway copy — breaking Wheels Channel pub/sub (subscriber counts stuck 0).
// Pattern: fetch the nested map into a local, mutate it, then re-read from the
// outer map (the Wheels Channel idiom: `subs = channels.get(chan); subs.put(...)`).
outer = CreateObject("java", "java.util.concurrent.ConcurrentHashMap").init();
inner = CreateObject("java", "java.util.concurrent.ConcurrentHashMap").init();
outer.put("chan", inner);
subs = outer.get("chan");
subs.put("sub1", "a");
subs.put("sub2", "b");
assert("nested put persists (re-read from outer)", outer.get("chan").size(), 2);
subs.remove("sub1");
assert("nested remove persists (re-read from outer)", outer.get("chan").size(), 1);
assertTrue("nested key still present", outer.get("chan").containsKey("sub2"));
subs.clear();
assert("nested clear persists (re-read from outer)", outer.get("chan").size(), 0);

// java.security.MessageDigest.isEqual compares two byte[] for CONTENT equality.
// RustCFML compared the values' as_string(), and Binary stringifies to the
// constant "<Binary>" — so ANY two byte arrays compared equal, defeating JWT
// signature / password verification. It now compares the raw bytes.
md = CreateObject("java", "java.security.MessageDigest");
assertTrue("isEqual: identical bytes", md.isEqual("abc".getBytes("UTF-8"), "abc".getBytes("UTF-8")));
assertFalse("isEqual: differing bytes", md.isEqual("abc".getBytes("UTF-8"), "abd".getBytes("UTF-8")));
assertFalse("isEqual: differing length", md.isEqual("abc".getBytes("UTF-8"), "abcd".getBytes("UTF-8")));

// java.util.TreeMap iterates its keys in SORTED (natural) order, regardless of
// insertion order — unlike a plain struct/HashMap. MockBox's normalizeArguments
// relies on this (`for(k in treeMap)`) to make its argument hash independent of
// call-site argument order; without sorted for-in, mocks with named args fail to
// match and `var x = mock()` assigns null -> "Variable undefined" (the Preside
// PresideObjectCloningService newId/relatedTo failures).
tm = CreateObject("java", "java.util.TreeMap").init({ objectName="o", propertyName="p", attributeName="a", zeta="z" });
forInOrder = "";
for ( k in tm ) { forInOrder = listAppend( forInOrder, k ); }
assert("TreeMap for-in iterates keys sorted", forInOrder, "attributeName,objectName,propertyName,zeta");
// keySet() is likewise sorted, and __java_* markers never leak into iteration.
assert("TreeMap keySet sorted", arrayToList( tm.keySet() ), "attributeName,objectName,propertyName,zeta");
// Order-independent serialization (the MockBox idiom): two maps with the same
// entries in different insertion order iterate identically.
tm2 = CreateObject("java", "java.util.TreeMap").init({ zeta="z", attributeName="a", propertyName="p", objectName="o" });
ser1 = ""; for ( k in tm  ) { ser1 &= k & "=" & tm[k]  & ";"; }
ser2 = ""; for ( k in tm2 ) { ser2 &= k & "=" & tm2[k] & ";"; }
assert("TreeMap iteration order-independent of insertion order", ser1, ser2);

suiteEnd();
</cfscript>
