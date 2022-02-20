# corne-xiao
Rust firmware for crkbd with Seeeduino Xiao

## Build and flash
Because of the limited amount of pins on the Seeeduino Xiao, two different versions of the firmware have to be flashed to both halves. Also, because of issues with the default bootloader, this firmware needs to be flashed via a programmer (e.g. Blackmagic Probe, etc.). It is still unclear as to why the bootloader doesn't work.

Build right-hand side:
```
cargo build --release --features right
```

Build left-hand side:
```
cargo build --release
```

## Adjust layout
Look into `layout.rs` and change your keymap.

## Build yourself
The schematic and pcb can be found in `corne-chocolate.zip`.
