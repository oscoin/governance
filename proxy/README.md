# Proxy

A background service that implements all business logic tying together the
radicle code collaboration protocol.

### Run

To start up the binary you can run: `cargo run`.
After that the API is served on `http://127.0.0.1:17246/docs`.


### Testing

Before running the test suite, download the test fixtures:

```sh
git submodule update --recursive
```

Then run tests as usual:

```sh
cargo test
```
