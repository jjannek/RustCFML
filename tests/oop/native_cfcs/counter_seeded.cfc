component extends="rust:Counter" {

    public counter_seeded function init(required numeric seed) {
        super(seed);
        return this;
    }

}
