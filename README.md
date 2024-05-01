# Prerequisites
- rustc
- cargo

https://www.rust-lang.org/tools/install

# How to run

```bash
$ cargo run -r <path_to_dns_records>
```
or
```bash
$ cargo build -r
$ ./target/release/dns-server <path_to_dns_records>
```

Note: server starting on port 5353