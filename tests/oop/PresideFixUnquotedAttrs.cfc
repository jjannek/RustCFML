/**
 * Regression helper: a `property` declaration that mixes quoted and UNQUOTED
 * attribute values (Lucee-legal), with a feature gate as the LAST attribute —
 * mirrors Preside's `website_applied_permission.benefit`. An unquoted value
 * (`required=false`, `ondelete=cascade`) must NOT terminate attribute parsing,
 * or the trailing `feature="…"` is silently dropped and the property survives
 * feature-disabled deletion (broke Preside serve-mode boot at RelationshipGuidance).
 */
component {
	property name="benefit" relationship="many-to-one" relatedto="website_benefit" required=false uniqueindexes="context_permission|4" ondelete="cascade" feature="websiteBenefits";
	property name="qty" type="numeric" maxlength=100 default=5 feature="someFeature";
}
