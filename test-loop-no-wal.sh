#!/bin/bash

cargo build --release

while true
do
    ./target/release/rocksdb-flush-safety --storage-dir "storage-no-wal"
done