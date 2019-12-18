/// Convenience function for setting env vars only if they are unset.
pub fn set_if_unset (key: &str, val: &str) {
    use std::env::{var,set_var};
    var(key).unwrap_or_else(|_| { set_var(key, val); var(key).unwrap() });
}

#[test]
fn test_set_if_unset() {
    match std::env::var("DUMMY_VALUE") {
        Ok(_val) => {
            // should be unset
            panic!("$DUMMY_VALUE should be unset.")
        },
        Err(_e) => {
            set_if_unset("DUMMY_VALUE", "hello world")
        }
    }
    match std::env::var("DUMMY_VALUE") {
        Ok(val) => { assert_eq!(val, "hello world") },
        Err(_e) => {
            panic!("$DUMMY_VALUE is still unset.")
        }
    }
}
