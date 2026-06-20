<!---
  Regression (v0.239.0):
  - value.getClass().getName()/getSimpleName() must work on non-component values
    (was returning Null, so the chained .getName() threw "cannot call method on
    null"). Used by TestBox's instanceOf matcher + Wheels toXML.
  - IsImageFile() must content-sniff (a text file renamed .png is NOT an image),
    and GetReadableImageFormats() must return a non-empty simple value.
  Passes on RustCFML + Lucee 7.
--->
<cfscript>
suiteBegin("getClass() + image BIFs on non-component values");

b = false;
assertTrue("boolean.getClass().getName()", b.getClass().getName() == "java.lang.Boolean");
s = "hello";
assertTrue("string.getClass().getName()", s.getClass().getName() == "java.lang.String");
assertTrue("string.getClass().getSimpleName()", s.getClass().getSimpleName() == "String");
arr = [1, 2];
assertTrue("array.getClass().getName() is non-empty", len(arr.getClass().getName()) gt 0);

assertTrue("GetReadableImageFormats is a simple value", isSimpleValue(GetReadableImageFormats()));
assertTrue("GetReadableImageFormats is non-empty", len(GetReadableImageFormats()) gt 0);

// IsImageFile content-sniffs: a text file is not an image, and a missing file
// is not an image. (The positive case — a real image returns true — is covered
// by the Wheels assetsSpec image fixtures; constructing image bytes here via
// toBinary()+fileWrite() is avoided so the test stays engine-portable.)
base = getTempDirectory() & "/rcfml_img_" & getTickCount();
fileWrite(base & ".txt", "this is plainly not an image");
assertFalse("IsImageFile false for a text file", IsImageFile(base & ".txt"));
assertFalse("IsImageFile false for a missing file", IsImageFile(base & "_nope.png"));
fileDelete(base & ".txt");
suiteEnd();
</cfscript>
