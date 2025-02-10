# Changelog

## [Unreleased](https://github.com/harmless-tech/rust-socks2/tree/main)

- (TODO) No longer depends on `byteorder`???
- (TODO) Move connect_timeout to a Config struct???
- (TODO) Read and Write timeout???

## [0.4.0](https://github.com/harmless-tech/rust-socks2/releases/tag/v0.4.0)

- Empty domain names will now error before being sent for Socks5.
- Remove all unwraps.
- New Error type that is returned wrapped in io::Error.
- Add timeout to connect and bind methods.
- TargetAddr derives Eq, PartialEq, and Display.
- Add features to split up code.
- Some socks functions now want a reference to the target addr.
- Use rust edition 2021.
- Switch from `winapi` crate to `windows-sys` crate.
- Other under the hood improvements.

## [Before Fork](https://github.com/sfackler/rust-socks)
