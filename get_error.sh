#!/bin/bash
cd /mnt/c/Users/ugosa/Desktop/Cursor\ SOFTWARE/IPPAN
/home/ugosa/.cargo/bin/cargo check --tests -p ippan-rpc > build_error.txt 2>&1
