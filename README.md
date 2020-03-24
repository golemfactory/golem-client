# golem-client ![Continuous integration](https://github.com/golemfactory/golem-client/workflows/Continuous%20integration/badge.svg)

Client for [Brass Golem](https://github.com/golemfactory/golem) Network.

## Subprojects

* actix-wamp ([api docs](
https://docs.rs/actix-wamp/latest/actix_wamp
)) - Asynchronous client library for [WAMP](https://wamp-proto.org/). 
* golem-rpc-api ([api docs](
https://docs.rs/golem-rpc-api/latest/golem_rpc_api/
)) - Typesafe binding for Brass Golem RPC services.
* golem-rpc-api-macro - Procedural macros for binding Brass Golem RPC endpoints. 
 
* golemcli - command line interface for Brass Golem (re)implemented in Rust.

## GolemCLI

### Install or Upgrade

On Unix:

```
curl -sSf https://golemfactory.github.io/golem-client/install/golemcli-update.sh | bash
```


### Compilation prerequisites (Windows)

This project builds under Windows (validated under VC toolchain), after following prerequisites are installed:

* `rustup upgrade` - get latest stable Rust toolset
* Install Perl (eg. https://www.activestate.com/products/activeperl/downloads/)
* Install OpenSSL (https://slproweb.com/products/Win32OpenSSL.html)
