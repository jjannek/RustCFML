<cfscript>
suiteBegin("Timezone (setTimeZone / getTimeZone / getTimeZoneInfo / dateConvert)");

// setTimeZone() sets the request timezone, which on Lucee persists for the whole
// request and would shift how LATER test files interpret createDateTime instants.
// Capture it up front and restore it at the end so this suite is side-effect-free.
// (getTimeZone() round-trips through setTimeZone() on both engines.)
originalTZ = getTimeZone();

// --- setTimeZone / getTimeZone round-trip ---
setTimeZone("America/New_York");
assert("getTimeZone reflects setTimeZone", getTimeZone(), "America/New_York");
setTimeZone("Europe/London");
assert("getTimeZone after re-set", getTimeZone(), "Europe/London");

// --- getTimeZoneInfo static display names (DST zones; names are constant) ---
nyInfo = getTimeZoneInfo("America/New_York");
assert("NY id", nyInfo.id, "America/New_York");
assert("NY timezone", nyInfo.timezone, "America/New_York");
assert("NY shortName", nyInfo.shortName, "EST");
assert("NY shortNameDST", nyInfo.shortNameDST, "EDT");
assert("NY name", nyInfo.name, "Eastern Standard Time");
assert("NY nameDST", nyInfo.nameDST, "Eastern Daylight Time");

lonInfo = getTimeZoneInfo("Europe/London");
assert("London shortName", lonInfo.shortName, "GMT");
assert("London shortNameDST", lonInfo.shortNameDST, "BST");
assert("London name", lonInfo.name, "Greenwich Mean Time");
assert("London nameDST", lonInfo.nameDST, "British Summer Time");

// --- getTimeZoneInfo numeric fields for NON-DST zones (constant year-round) ---
// UTC: zero offset, never in DST.
utc = getTimeZoneInfo("UTC");
assert("UTC offset", utc.offset, 0);
assert("UTC utcTotalOffset", utc.utcTotalOffset, 0);
assert("UTC utcHourOffset", utc.utcHourOffset, 0);
assert("UTC utcMinuteOffset", utc.utcMinuteOffset, 0);
assertFalse("UTC not DST", utc.isDSTon);

// Asia/Kolkata: +05:30, no DST — exercises the half-hour minute field.
kol = getTimeZoneInfo("Asia/Kolkata");
assert("Kolkata offset (+5:30 = 19800s)", kol.offset, 19800);
assert("Kolkata utcTotalOffset", kol.utcTotalOffset, -19800);
assert("Kolkata utcHourOffset", kol.utcHourOffset, -5);
assert("Kolkata utcMinuteOffset", kol.utcMinuteOffset, -30);
assertFalse("Kolkata not DST", kol.isDSTon);

// Asia/Tokyo: +09:00, no DST.
tok = getTimeZoneInfo("Asia/Tokyo");
assert("Tokyo offset", tok.offset, 32400);
assert("Tokyo utcHourOffset", tok.utcHourOffset, -9);

// --- dateConvert honors the set zone, DST decided by the date itself ---
setTimeZone("America/New_York");
// 2026-06-22 is summer -> EDT (UTC-4): 12:00 local == 16:00 UTC.
assert(
    "dateConvert local2utc summer (EDT -4)",
    dateConvert("local2utc", createDateTime(2026, 6, 22, 12, 0, 0)),
    createDateTime(2026, 6, 22, 16, 0, 0)
);
assert(
    "dateConvert utc2local summer (EDT -4)",
    dateConvert("utc2local", createDateTime(2026, 6, 22, 16, 0, 0)),
    createDateTime(2026, 6, 22, 12, 0, 0)
);
// 2026-01-22 is winter -> EST (UTC-5): 12:00 local == 17:00 UTC.
assert(
    "dateConvert local2utc winter (EST -5)",
    dateConvert("local2utc", createDateTime(2026, 1, 22, 12, 0, 0)),
    createDateTime(2026, 1, 22, 17, 0, 0)
);

// Non-DST zone is season-independent.
setTimeZone("Asia/Kolkata");
assert(
    "dateConvert local2utc Kolkata (+5:30)",
    dateConvert("local2utc", createDateTime(2026, 6, 22, 12, 0, 0)),
    createDateTime(2026, 6, 22, 6, 30, 0)
);

// --- invalid timezone fails loudly ---
assertThrows("setTimeZone unknown id throws", function() {
    setTimeZone("Not/AZone");
});
assertThrows("getTimeZoneInfo unknown id throws", function() {
    getTimeZoneInfo("Not/AZone");
});

// Restore the request timezone so subsequent test files are unaffected.
setTimeZone(originalTZ);

suiteEnd();
</cfscript>
