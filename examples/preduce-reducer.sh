#!/usr/bin/env bash

TMP=$(mktemp -d || exit 2)
cp -f $1 $TMP/testcase.html || exit 2

reducewrap "( timeout 8s /shared/dev/rust/servo/mach run -rz -fx -Z replace-surrogates {}/testcase.html ) 2>&1 | grep -m 1 'multiply with overflow'" $TMP/testcase.html -C 1
EXIT=$?
rm -rf $TMP/
exit $EXIT
