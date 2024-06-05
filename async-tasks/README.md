# Async

A tiny runtime that allows spawning tasks dynamically. Nothing production ready, just a trivial implementation to learn the limits of the `Future`s + `async/await` feature of the language. Kind of a micro-single-threaded-`tokio.rs`.

## Run

```console
cargo run
```

it simply runs several tasks and produces output like this:

```console
[src/main.rs:167] "async 1 {" = "async 1 {"
[src/main.rs:178] "async 2 {" = "async 2 {"
[src/main.rs:189] "async 3 {" = "async 3 {"
[src/main.rs:201] "async 4 {" = "async 4 {"
[src/main.rs:206] "async 4 }" = "async 4 }"
[src/main.rs:212] "async 5 {" = "async 5 {"
[src/main.rs:213] "async 5 }" = "async 5 }"
[src/main.rs:220] "async 6 {" = "async 6 {"
[src/main.rs:226] "async 6 }" = "async 6 }"
[src/main.rs:232] "async 8 {" = "async 8 {"
unhandled error result: error in a task
[src/main.rs:222] "async 7 {" = "async 7 {"
[src/main.rs:223] "async 7 }" = "async 7 }"
[src/main.rs:170] "async 1 }" = "async 1 }"
[src/main.rs:193] "async 3 }" = "async 3 }"
[src/main.rs:181] "async 2 }" = "async 2 }"
```
