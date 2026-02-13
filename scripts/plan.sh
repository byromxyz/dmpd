#!/bin/bash

# Build the project
RUSTFLAGS="-Awarnings" cargo build

clear

# Delete all files in examples which do not end with .mpd
find ./examples -type f ! -name "*.mpd" -delete

# Run the built binary against each file
for file in ./examples/*.mpd; do

    if [[ "$file" == *"9-static-long.mpd" ]]; then
        echo -e "\nRunning: $file--max-duration-ms=360000 --slice"
        RUST_BACKTRACE=1 ./target/debug/$(basename $(pwd)) "$file" --max-duration-ms=360000 --slice
        echo -e "\nRunning: $file--max-duration-ms=360000 --slice -p"
        RUST_BACKTRACE=1 ./target/debug/$(basename $(pwd)) "$file" --max-duration-ms=360000 --slice -p
        continue
    fi

    if [[ "$file" == *"10-publish-gap.mpd" ]]; then
        echo -e "\nRunning: $file--max-duration-ms=20000"
        RUST_BACKTRACE=1 ./target/debug/$(basename $(pwd)) "$file" --max-duration-ms=20000
        echo -e "\nRunning: $file--max-duration-ms=20000 -p"
        RUST_BACKTRACE=1 ./target/debug/$(basename $(pwd)) "$file" --max-duration-ms=20000 -p
        continue
    fi

    echo -e "\nRunning: $file --max-duration-ms=-1"
    RUST_BACKTRACE=1 ./target/debug/$(basename $(pwd)) "$file" --max-duration-ms=-1
    echo -e "\nRunning: $file --max-duration-ms=-1 -p"
    RUST_BACKTRACE=1 ./target/debug/$(basename $(pwd)) "$file" --max-duration-ms=-1 -p
done
