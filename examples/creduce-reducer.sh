#!/usr/bin/env bash
# Assumes this script is ran inside creduce's /tmp directory, with a ./testcase.html file

reducewrap "( timeout 4s /shared/dev/rust/servo/mach run -rz -fx -Z replace-surrogates {}/testcase.html ) 2>&1 | grep -m 1 'Option::unwrap'" ./testcase.html -C 1
