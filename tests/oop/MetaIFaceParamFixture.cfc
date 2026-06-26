/**
 * Interface fixture whose method declares a required, typed parameter — its
 * metadata `functions[1].parameters` must surface name/type/required so MockBox
 * can regenerate the argument list (issue #205).
 */
interface {
	public void function configure(required string id);
}
