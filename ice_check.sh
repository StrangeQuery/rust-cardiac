#!/bin/sh
# rustc's internal errors in a run. A delayed bug is reported when rustc's session ends, after the backend has
# written its output, so a test can pass with one; rustc writes rustc-ice-*.txt where it ran (the repo, the test
# programs, the examples, the fuzzer's packages). Fails, naming them, if any is newer than the file MARK.
#
#     sh ice_check.sh MARK
cd "$(dirname "$0")"
ices=$({ find . -maxdepth 1 -name 'rustc-ice-*' -newer "$1"; find tests examples target/fuzz -maxdepth 3 -name 'rustc-ice-*' -newer "$1"; } 2>/dev/null)
[ -z "$ices" ] && exit 0
echo "rustc reported internal errors:"
echo "$ices"
exit 1
