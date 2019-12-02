# Golem RPC macros [![build-status-travis]][travis] [![crates.io-version]][crates.io]

[build-status-travis]: https://travis-ci.org/golemfactory/golem-client.svg?branch=master
[travis]: https://travis-ci.org/golemfactory/golem-client
[crates.io-version]: http://meritbadge.herokuapp.com/golem-rpc-macros
[crates.io]: https://crates.io/crates/golem-rpc-macros

Procedural macros for binding Brass Golem RPC endpoints.
Currently facilitates automated generation of settings RPC bindings.

Example code
```Rust
        /// Max memory size
        #[unit = "kB"]
        #[check("v >= 1048576")]
        max_memory_size: usize,
```

For more usage examples see [settings module](
../golem-rpc-api/src/settings.rs
) in golem-rpc-api.