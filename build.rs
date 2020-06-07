use vergen::{ConstantsFlags, generate_cargo_keys};

fn main() {
    generate_cargo_keys(
        ConstantsFlags::SHA_SHORT
        | ConstantsFlags::SEMVER
        | ConstantsFlags::COMMIT_DATE
        | ConstantsFlags::REBUILD_ON_HEAD_CHANGE
    ).unwrap();
}
