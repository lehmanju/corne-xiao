# corne-xiao
Rust firmware for crkbd with Seeeduino Xiao

## Build and flash
Because of the limited amount of pins on the Seeeduino Xiao, there are two different versions of the firmware, one for each half. Flashing is done via `cargo hf2`.

Install `cargo-hf2`:

```
cargo install cargo-hf2
```

Build and flash right-hand side:
```
cargo hf2 --release --features right
```

Build and flash left-hand side:
```
cargo hf2 --release
```

## Adjust layout
Look into `layout.rs` and change your keymap.

## Build yourself
The schematic and pcb can be found in `corne-chocolate.zip`.
