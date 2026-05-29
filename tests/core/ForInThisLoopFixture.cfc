component {

    // The exact shape Wheels' public/Application.cfc uses to scan plugin
    // folders:  for (this.wheels.folder in this.wheels.pluginFolders) { ... }
    // A `this`-headed for-in loop variable. Lucee/ACF/BoxLang accept it;
    // RustCFML 0.20.2 fails to PARSE it ("Expected Semicolon, found In"),
    // which degrades this whole component at instantiation.
    function scanArray() {
        this.wheels = {folder: "", pluginFolders: ["x", "y", "z"]};
        this.joined = "";
        for (this.wheels.folder in this.wheels.pluginFolders) {
            this.joined = this.joined & this.wheels.folder;
        }
        return this.joined;
    }

    // The value a `this`-scoped loop variable holds after the loop ends
    // (engine-consistent across Lucee/ACF/BoxLang: the final element).
    function lastFolder() {
        this.wheels = {folder: "", pluginFolders: ["x", "y", "z"]};
        for (this.wheels.folder in this.wheels.pluginFolders) {
        }
        return this.wheels.folder;
    }

}
