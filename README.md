# golem-client

[Project roadmap](https://docs.google.com/document/d/1h1pUB-LT6YwozfqX9rAO7vrgzM5CaGGr9WsePeZ95C8) 

## Subprojects

* actix-wamp ([api docs](https://golemfactory.github.io/golem-client/latest/actix_wamp/index.html)) - Web Application Messaging Protocol implementation. 
* golem-rpc-api ([api docs](https://golemfactory.github.io/golem-client/latest/golem_rpc_api/index.html)) - Typesafe binding for golemapp rpc services. 
* golemcli - command line interface for golem. reimplementation.

## Prerequisites (Windows)

This project builds under Windows (validated under VC toolchain), after following prerequisites are installed:

* `rustup upgrade` - get latest stable Rust toolset
* Install Perl (eg. https://www.activestate.com/products/activeperl/downloads/)
* Install OpenSSL (https://slproweb.com/products/Win32OpenSSL.html)
