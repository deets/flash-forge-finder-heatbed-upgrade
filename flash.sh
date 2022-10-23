#!/bin/bash
cargo espflash --release --features ttgo --target xtensa-esp32s2-espidf --monitor --speed 460800
