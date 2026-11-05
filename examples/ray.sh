#!/bin/sh
# Ray-traces a mirror ball over a red and cream checkerboard on CARDIAC: a 96 x 48 picture, ray.ppm (and ray.png,
# where sips is there to make it), of one frame of the ball bouncing and swaying while the floor scrolls under it.
#
# This script is the host. It deals each machine in ray/ its cards, passes what one outputs on to the next, looks
# each colour code up in a fixed palette, as a display does, and writes the picture. Everything that shows is worked
# out on CARDIAC: burn runs the ray/ decks for the ball's place, for every row of sky and floor, and for every pixel
# of the ball, about 600 rays a frame.
#
#     cargo cardiac build && sh ray.sh
#
# The settings, all optional:
#   FRAME   which frame of the 120 to draw, 0 to 119 (0). The ball bounces every 24 frames and sways from side to
#           side once in 120
#   OUT     the picture's file (ray.ppm)
#   BURN    another burn
set -e
cd "$(dirname "$0")"
BURN=${BURN:-burn}
FRAME=${FRAME:-0}
OUT=${OUT:-ray.ppm}
case $FRAME in *[!0-9]*|"") echo "ray.sh: FRAME is a frame number, 0 to 119"; exit 2;; esac
[ "$FRAME" -le 119 ] || { echo "ray.sh: FRAME is a frame number, 0 to 119"; exit 2; }
command -v "$BURN" >/dev/null || { echo "ray.sh: needs burn, from cardiaq: put it on the PATH, or set BURN"; exit 1; }
for m in sky zrow floor span norm bounce hit compose sway hop; do
  [ -f ray/$m.cqb ] || { echo "ray.sh: build it first: cargo cardiac build"; exit 1; }
done
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
run() {   # MACHINE CARD...: runs the machine's deck on those cards and gives what it outputs, on one line
  m=$1; shift
  "$BURN" ray/$m.cqb -- "$@" 2>/dev/null | tr '\n' ' '
}

# The frame: sway is dealt the frame's line of the sine table and works out the ball's x; hop works out its height
# and the floor's scroll from the frame number.
sine=$(grep -v '^#' ray/sine120.txt | sed -n "$((FRAME + 1))p")
bx=$(run sway $sine)
set -- $(run hop $FRAME)
bh=$1 xs=$2 zs=$3

y=0
while [ $y -lt 48 ]; do
  # The background: sky gives the row's colours, or -1 for a floor row, which zrow and floor draw.
  background=$(run sky $y)
  if [ "$background" = "-1 " ]; then
    background=$(run floor $(run zrow $y $zs) $xs $bx)
  fi
  # The ball: span gives the pixels it covers on this row; each one's ray goes through norm and bounce, and on to
  # hit when bounce says it reaches the floor.
  set -- $(run span $y $bx $bh)
  start=$1 count=$2
  ball=""
  if [ "$count" -gt 0 ]; then
    b=$3; shift 3
    for a in "$@"; do
      set -- $(run norm $a $b)
      c=$1 rx=$2 ry=$3 rz=$4
      set -- $(run bounce $rx $ry $b $bh)
      if [ "$1" -ge 0 ]; then code=$1; else code=$(run hit $2 $rx $rz $a $c $bx $xs $zs); fi
      ball="$ball $code"
    done
  fi
  run compose $start $count $ball $background >> "$work/codes"
  echo >> "$work/codes"
  y=$((y + 1))
done

# The picture: each code CARDIAC gave, looked up in the palette. 0-2 the sky's bands, 3 haze, 4 and 5 the checkers,
# 6 and 7 the checkers in shadow, 8 and 9 the sky in the ball, 10 and 11 the checkers in the ball, 12 haze in the
# ball, 13 the light.
awk -v out="$OUT" 'BEGIN {
  split("40 60 150|80 120 200|150 180 230|170 180 200|200 40 40|235 225 200|110 25 25|130 125 110|110 150 220|170 195 235|160 40 45|205 200 180|150 160 180|255 255 255", pal, "|")
  print "P3\n96 48\n255" > out
}
{ for (i = 1; i <= NF; i++) print pal[$i + 1] > out }' "$work/codes"
rm -f "${OUT%.ppm}.png"
if command -v sips >/dev/null; then sips -s format png "$OUT" --out "${OUT%.ppm}.png" >/dev/null 2>&1 || true; fi
echo "ray.sh: wrote $OUT (frame $FRAME, every ray on CARDIAC)"
