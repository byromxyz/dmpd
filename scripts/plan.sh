#!/bin/bash

for file in ./examples/*.mpd; do
    RUSTFLAGS="-Awarnings" cargo run "$file"
    RUSTFLAGS="-Awarnings" cargo run "$file" -p
done
cd
