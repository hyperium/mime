# mime

[![crates.io](https://img.shields.io/crates/v/mime.svg)](https://crates.io/crates/mime)
[![docs.rs](https://docs.rs/mime/badge.svg)](https://docs.rs/mime)

Support MIME (HTTP Media Types) as strong types in Rust.

## Usage

```rust
fn main() {
    // common types are constants
    let text = mime::TEXT_PLAIN;

    // deconstruct Mimes to match on them
    match (text.type_(), text.subtype()) {
        (mime::TEXT, mime::PLAIN) => {
            // plain text!
        }
        (mime::TEXT, _) => {
            // structured text!
        }
        _ => {
            // not text!
        }
    }
}
```

## Documentation

Read the online documents at https://docs.rs/mime.

## License

This project is licensed under [The MIT License](LICENSE).
