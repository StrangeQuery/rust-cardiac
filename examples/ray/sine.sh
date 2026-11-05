#!/bin/sh
# Writes the sine table sway.rs is dealt from, sine120.txt: 999 sin of k 120ths of a turn for k = 0 to 119, rounded,
# one a line. It is data, like a printed table: the frame's line of it is the card `ray.sh` gives sway. Pi is
# atan2(0, -1), the nearest a double holds, so 150 degrees, where 999 sin is 499.5 to within a hair, rounds to 499.
#
#     sh ray/sine.sh > ray/sine120.txt
awk 'BEGIN {
  print "# k (120ths of a turn): 999 sin"
  for (k = 0; k < 120; k++) { v = 999 * sin(2 * atan2(0, -1) * k / 120) + 0.5; i = int(v); if (i > v) i--; print i }
}'
