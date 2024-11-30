# PTY

Trivial telnet server in Rust.

Heavily inspired by Ivan Velichko's [post](https://iximiuz.com/en/posts/linux-pty-what-powers-docker-attach-functionality/) and [source code](https://github.com/iximiuz/ptyme/tree/master).

More useful links:

- [https://stackoverflow.com/questions/74284202/how-to-use-a-pseudo-terminal-returned-from-posix-openpt]
- [https://github.com/dzervas/netcatty]
- [https://github.com/nelhage/reptyr]

## Run

```bash
# in one terminal
cargo run
# in another
./attach.sh
```
