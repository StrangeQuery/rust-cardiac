#!/bin/sh
# Makes a height map for voxel.sh to fly over, STYLE.pgm: a 256 x 256 greyscale image, each pixel's brightness the
# land's height there (0 the lowest, 255 the highest). It starts as fractal noise, a few layers of smooth random
# bumps, each layer twice as fine and half as tall as the one before, which is what makes it look like land; then
# the style shapes it:
#   hills     the noise as it is: rolling hills, a few lakes, a few peaks
#   islands   the low ground pushed lower, so the sea floods most of it and the hills stand out of it as islands
#   alps      each layer of the noise folded at its middle height into a sharp ridge, the valleys deepened: mountains
#   sines     no noise: a few sine waves added up, a lake in a valley with green hills and snowy peaks round it
#
# PGM is the plainest image format there is: a header (P2, the width and height, the brightest value), then the
# pixels as numbers, row by row. Any image editor that saves PGM can make a height map of your own: paint it, save
# it (256 x 256, plain), and give it to voxel.sh: MAP=mine.pgm sh voxel.sh.
#
#     sh terrain.sh [STYLE [SEED]]       # hills, islands, alps or sines: hills.pgm, islands.pgm ...
set -e
cd "$(dirname "$0")"
STYLE=${1:-hills}
case $STYLE in hills|islands|alps|sines) ;; *) echo "terrain.sh: STYLE is hills, islands, alps or sines"; exit 2;; esac
if [ "$STYLE" = sines ]; then
  # The waves, for heights of 12 to 99, sampled 3.5 units of ground a pixel, x across (right to left: the camera
  # faces down the map) and y down the map. It says so in the file, with how tall to draw it: relief 0.75, which
  # draws them as flat as the first landscapes were.
  awk 'BEGIN {
    N = 256
    print "P2\n# sines.pgm: a height map for voxel.sh, made by terrain.sh (sines)\n# scale: unit 3.5 relief 0.75\n" N " " N "\n255" > "sines.pgm"
    for (r = 0; r < N; r++) {
      line = ""
      for (q = 0; q < N; q++) {
        x = (128 - q) * 3.5; y = r * 3.5
        h = 44 + 16 * sin(x / 23) + 13 * cos(y / 17 + x / 31) + 9 * sin((x + 2 * y) / 11) + 5 * cos(x / 5 - y / 7)
        if (h < 12) h = 12; if (h > 99) h = 99
        line = line (q ? " " : "") int((h - 12) * 255 / 87 + 0.5)
      }
      print line > "sines.pgm"
    }
  }'
  echo "terrain.sh: wrote sines.pgm"
  exit 0
fi
awk -v seed="${2:-1968}" -v style="$STYLE" 'BEGIN {
  srand(seed); N = 256
  for (o = 0; o < 5; o++) {                       # each layer: a grid of random heights, wrapping round the edges
    g = 4 * 2 ^ o
    for (j = 0; j < g; j++) for (i = 0; i < g; i++) R[o, j, i] = rand()
  }
  lo = 1e9; hi = -1e9
  for (y = 0; y < N; y++) for (x = 0; x < N; x++) {
    h = 0; amp = 1
    for (o = 0; o < 5; o++) {
      g = 4 * 2 ^ o; fx = x * g / N; fy = y * g / N
      i = int(fx); j = int(fy); tx = fx - i; ty = fy - j
      tx = tx * tx * (3 - 2 * tx); ty = ty * ty * (3 - 2 * ty)    # smooth between the grid points
      a = R[o, j, i]; b = R[o, j, (i + 1) % g]; c = R[o, (j + 1) % g, i]; d = R[o, (j + 1) % g, (i + 1) % g]
      v = a + (b - a) * tx + (c - a) * ty + (a - b - c + d) * tx * ty
      if (style == "alps") v = (1 - (v < 0.5 ? 1 - 2 * v : 2 * v - 1)) ^ 2   # each layer folded into ridges
      h += amp * v
      amp /= 2
    }
    H[y, x] = h; if (h < lo) lo = h; if (h > hi) hi = h
  }
  out = style ".pgm"
  print "P2\n# " out ": a height map for voxel.sh, made by terrain.sh (" style ", seed " seed ")\n" N " " N "\n255" > out
  for (y = 0; y < N; y++) {
    line = ""
    for (x = 0; x < N; x++) {
      n = (H[y, x] - lo) / (hi - lo)
      if (style == "islands") n = n ^ 2.2
      if (style == "alps") n = n ^ 1.4
      line = line (x ? " " : "") int(255 * n + 0.5)
    }
    print line > out
  }
}'
echo "terrain.sh: wrote $STYLE.pgm"
