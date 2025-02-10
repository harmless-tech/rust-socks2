# socks2

[![Crates.io Version](https://img.shields.io/crates/v/socks2?style=flat-square&color=blue)](https://crates.io/crates/socks2)
[![docs.rs](https://img.shields.io/docsrs/socks2?style=flat-square)](https://docs.rs/socks2)
[![Crates.io MSRV](https://img.shields.io/crates/msrv/socks2?style=flat-square)](https://www.rust-lang.org/tools/install)

SOCKS proxy support for Rust.

A fork of [sfackler/rust-socks](https://github.com/sfackler/rust-socks).

See [changes](CHANGELOG.md).

## Using

```cargo add socks2```

```toml
[dependencies]
socks2 = "0.4"
```

### Features

#### client

```toml
[dependencies]
socks2 = { version = "0.4", default-features = false, features = ["client"] }
```

```rust
use socks2::Socks4Stream;
use socks2::Socks5Stream;
use std::io::Write;

let mut connection = Socks4Stream::connect(PROXY, &TARGET, "userid").unwrap();
let buf = [126_u8; 50]
connection.write(&buf);

let mut connection = Socks5Stream::connect(PROXY, &TARGET).unwrap();
let buf = [126_u8; 50]
connection.write(&buf);
```

#### bind

```toml
[dependencies]
socks2 = { version = "0.4", default-features = false, features = ["bind"] }
```

```rust
use socks2::Socks4Listener;
use socks2::Socks5Listener;

let mut connection = Socks4Listener::bin(PROXY, &TARGET, "userid")
    .unwrap()
    .accept();

let mut connection = Socks5Listener::bind(PROXY, &TARGET)
    .unwrap()
    .accept();
```

#### udp

```toml
[dependencies]
socks2 = { version = "0.4", default-features = false, features = ["udp"] }
```

```rust
use socks2::Socks5Datagram;
use std::io::Write;

let mut connection = Socks5Datagram::bind(PROXY, &TARGET).unwrap();
let buf = [126_u8; 50]
connection.send_to(&buf, &OTHER_ADDR);
```
