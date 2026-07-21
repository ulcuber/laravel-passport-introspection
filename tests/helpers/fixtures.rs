use std::fs;

use once_cell::sync::Lazy;

pub static FIXTURES: Lazy<Fixtures> = Lazy::new(|| Fixtures::load());

pub struct Fixtures {
    pub jwt_public_key: String,
    pub jwt_private_key: String,
}

impl Fixtures {
    fn load() -> Self {
        Self {
            jwt_public_key: load_fixture("oauth-public.key"),
            jwt_private_key: load_fixture("oauth-private.key"),
        }
    }
}

fn load_fixture(name: &str) -> String {
    fs::read_to_string(format!("tests/fixtures/{}", name))
        .expect(&format!("Failed to read fixture: {}", name))
}
