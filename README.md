rust port of https://github.com/DominicBreuker/pspy with support for process monitoring over dbus.

cross is needed for cross-compilation due to the usage of dbus-rs lib within rspy.
```
cargo install cross --git https://github.com/cross-rs/cross
cross build --target x86_64-unknown-linux-musl --release
```
