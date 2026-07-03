use transport_core::{DefaultGreeter, Error, Greeter};

#[test]
fn greet_returns_expected_output() {
    let g = DefaultGreeter;
    assert_eq!(g.greet("integration").unwrap(), "hello, integration");
}

#[test]
fn greet_rejects_empty_name() {
    let g = DefaultGreeter;
    let err = g.greet("").unwrap_err();
    matches!(err, Error::InvalidInput(_));
}
