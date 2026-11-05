#!/bin/sh
# Draws a landscape with voxel.rs on CARDIAC: a 96 x 96 picture, voxel.ppm, of the land in a height map, MAP
# (hills.pgm, which terrain.sh makes when it isn't there); and the map with the strips CARDIAC was given drawn on
# it, voxel_map.ppm (and each as a .png, where sips is there to make it).
#
# This script is the host, and does what a host does: it reads the height map, lays out where each slice of it
# falls under the screen's columns, and paints. The projection, where on the screen each piece of land shows, is
# worked out on CARDIAC: burn runs voxel.cqb once for each slice, near to far, and each column keeps the land
# that shows over what's in front of it.
#
#     cargo cardiac build --bin voxel && sh voxel.sh
#
# The settings, all optional:
#   MAP       the height map (hills.pgm)
#   CAMX      the pixel of the map the camera stands over, across (64) and down (160)
#   CAMY
#   LOOK      which way the camera faces on the map: up (north, on a real map), right (east), down (south), left
#             (west), or a compass bearing in degrees, 0 up the map and 90 to the right
#   ABOVE     how high the camera is over the ground beneath it, in the map's brightness, 0 to 255 (20)
#   CAMZ      or how high it is outright, in the map's brightness: for standing at a lake's edge, say
#   HORIZON   the row the horizon crosses the middle of the screen at, 0 to 95 (40): lower tilts the camera up
#   BANK      how far the camera is banked, in rows the horizon drops across the screen (8; 0 is level)
#   ZOOM      how far the lens is zoomed in: 1 sees 90 degrees across, 2 about 53, as a camera's normal lens (1)
#   NEAR      how far out the first slice is, in pixels of the map (2): farther hides the steps the heights
#             come in, right in front of the camera
#
# The heights CARDIAC is given run 0 to 49, black to white, so that 20 (camera - h) always fits on a card wherever
# the camera is, from the lowest ground to the highest; UNIT says how much ground a pixel is on that scale.
#   UNIT      how much ground a pixel of the map is, on the heights' scale of 0 to 49: smaller makes the land
#             steeper. A real map says how many metres a pixel is and which heights it spans, in a comment
#             ("# ground: 62.0 m a pixel; heights 529 m to 2523 m"), and then UNIT is worked out from it, so the land is
#             as steep as it really is; a made-up map can say its scale ("# scale: unit 3.5 relief 0.75"); else 1.75
#   RELIEF    how tall to draw the heights: 1, as they are, or the map's own (a made-up map's "relief" above)
#   FAR       how far out the last slice is, in pixels of the map (190). A real map doesn't repeat past its edges,
#             so put the camera at least that far from the edge it looks toward
#   WATER     below this brightness is water, flat (77; 0 for none)
#   FLAT      1: a patch of the map that is perfectly flat is water too, as a lake is in real elevation data (0)
#
# A map can also name lakes too small for its pixels to show, in comments: "# lake: 28.6 241.5 2 Mirror Lake", the
# pixel of its middle and its radius in pixels. Inside, the land is water, flat at the lowest level there.
#   PALETTE   land (water, sand, grass, dark grass, rock, snow), mountain (water, forest, meadow, rock, snow) or
#             desert (red rock to pale stone, juniper on top)
#   SLICES    how many slices (120)
#   BURN      another burn
set -e
cd "$(dirname "$0")"
BURN=${BURN:-burn}
SLICES=${SLICES:-120}
CAMX=${CAMX:-64}
CAMY=${CAMY:-160}
MAP=${MAP:-hills.pgm}
LOOK=${LOOK:-down}
ABOVE=${ABOVE:-20}
CAMZ=${CAMZ:-}
ZOOM=${ZOOM:-1}
BANK=${BANK:-8}
HORIZON=${HORIZON:-40}
NEAR=${NEAR:-2}
WATER=${WATER:-77}
PALETTE=${PALETTE:-land}
UNIT=${UNIT:-}
RELIEF=${RELIEF:-}
FAR=${FAR:-190}
FLAT=${FLAT:-0}
case $LOOK in up) LOOK=0;; right) LOOK=90;; down) LOOK=180;; left) LOOK=270;; esac
case $LOOK in *[!0-9.]*|"") echo "voxel.sh: LOOK is up, right, down, left or a bearing in degrees"; exit 2;; esac
case $PALETTE in land|mountain|desert) ;; *) echo "voxel.sh: PALETTE is land, mountain or desert"; exit 2;; esac
[ -f voxel.cqb ] || { echo "voxel.sh: build it first: cargo cardiac build --bin voxel"; exit 1; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# Each slice's deck: its distance d (growing as it goes, so the near land is fine and the far land coarse), the
# camera's height, then for each column the horizon's row (it tilts a little: the camera is banking) and the
# height-map cell under it, 10 h + c.
# The land: MAP, a height map (terrain.sh makes them; any 256 x 256 plain PGM will do), brightness its height,
# blended between the four pixels round a point; the map repeats past its edges. The camera stands over the pixel
# CAMX, CAMY, ABOVE over the ground there, facing LOOK. The colour comes from the height.
[ -f "$MAP" ] || case $MAP in hills.pgm|islands.pgm|alps.pgm|sines.pgm) sh terrain.sh "${MAP%.pgm}";; *) echo "voxel.sh: no $MAP"; exit 1;; esac
LAND='function load(   n, w, i, k) {
  n = 0
  while ((getline line < map) > 0) {
    if (line ~ /^# ground: /) { split(line, g, " "); mpp = g[3]; mlo = g[8]; mhi = g[11] }
    if (line ~ /^# scale: /) { split(line, g, " "); munit = g[4]; mrelief = g[6] }
    if (line ~ /^# lake: /) { split(line, g, " "); lakes++; LQ[lakes] = g[3]; LR[lakes] = g[4]; LS[lakes] = g[5] }
    sub(/#.*/, "", line); k = split(line, w, " ")
    for (i = 1; i <= k; i++) T[n++] = w[i]
  }
  close(map)
  if (T[0] != "P2" || T[1] != 256 || T[2] != 256) { print "voxel.sh: " map " is not a plain 256 x 256 PGM" > "/dev/stderr"; exit 1 }
  top = T[3]
  # units of ground a pixel: as given; or from a real map, its metres a pixel over its metres a unit of height
  u = unit != "" ? unit : mpp ? mpp / ((mhi - mlo) / 49) : munit ? munit : 1.75
  # the way the camera faces, and its right, on the map
  b = look * 3.14159265 / 180; fx = sin(b); fy = -cos(b)
  rx = -fy; ry = fx
  for (k = 1; k <= lakes; k++) {                # each lake, flat at the lowest level inside it
    LV[k] = 255
    for (j = int(LR[k] - LS[k]); j <= LR[k] + LS[k] + 1; j++) for (i = int(LQ[k] - LS[k]); i <= LQ[k] + LS[k] + 1; i++)
      if ((i - LQ[k]) ^ 2 + (j - LR[k]) ^ 2 <= LS[k] ^ 2 && px(j, i) < LV[k]) LV[k] = px(j, i)
  }
  tall = relief != "" ? relief : mrelief ? mrelief : 1
}
function px(r, q) { return T[4 + ((r % 256 + 256) % 256) * 256 + (q % 256 + 256) % 256] / top * 255 }
function flat(j, i,   v) {                    # a perfectly flat patch: a lake, in real elevation data
  v = px(j, i)
  return px(j, i + 1) == v && px(j + 1, i) == v && px(j + 1, i + 1) == v && px(j - 1, i - 1) == v && px(j + 2, i + 2) == v
}
# The land at the place a column of a slice falls on the map, d units ahead of the camera and x to its right: its
# height h (0 to 49) and colour c.
function land(x, d,   mx, my, i, j, tx, ty, v) {
  mx = camx + (x * rx + d * fx) / u; my = camy + (x * ry + d * fy) / u
  i = int(mx); j = int(my); if (mx < i) i--; if (my < j) j--
  tx = mx - i; ty = my - j
  v = px(j, i) * (1 - tx) * (1 - ty) + px(j, i + 1) * tx * (1 - ty) + px(j + 1, i) * (1 - tx) * ty + px(j + 1, i + 1) * tx * ty
  water = v < water_below || (flats && flat(j, i))
  for (k = 1; k <= lakes; k++) if ((mx - LQ[k]) ^ 2 + (my - LR[k]) ^ 2 <= LS[k] ^ 2) { water = 1; v = LV[k] }
  if (v < water_below) v = water_below
  h = v * 49 / 255
  # the colour: water, then five bands of height
  c = water ? 0 : v < 87 ? 1 : v < 128 ? 2 : v < 165 ? 3 : v < 200 ? 4 : 5
}'
awk -v slices="$SLICES" -v work="$work" -v camx="$CAMX" -v camy="$CAMY" -v map="$MAP" -v look="$LOOK" -v above="$ABOVE" -v camz="$CAMZ" -v water_below="$WATER" -v palette="$PALETTE" -v unit="$UNIT" -v flats="$FLAT" -v farpx="$FAR" -v relief="$RELIEF" -v zoom="$ZOOM" -v bank="$BANK" -v horizon0="$HORIZON" -v nearpx="$NEAR" "$LAND"' BEGIN {
  load(); land(0, 0); camera = camz != "" ? camz * 49 / 255 : h + above * 49 / 255; if (camera > 49) camera = 49; far = farpx * u; near = nearpx * u
  for (s = 1; s <= slices; s++) {
    di = int(near * (far / near) ^ ((s - 1) / (slices - 1)) + 0.5)   # each a little farther out than the last
    deck = work "/slice" s
    # CARDIAC is told the distance as d 20 / 48: the program puts 20 rows to a unit of height over the distance,
    # and the slice spreads 48 columns to a unit across it, so this is what makes a hill as tall as it is wide;
    # over the relief, to draw the heights taller or flatter than they are, and over the zoom, which narrows the
    # slice across the screen and so makes everything on it bigger, as tall as it is wide
    dc = int(di * 20 / 48 / tall / zoom + 0.5); if (dc < 1) dc = 1
    printf "%d %d", dc, int(camera + 0.5) > deck
    for (col = 0; col < 96; col++) {
      x = (col - 48) * di / 48 / zoom                # across, as wide as the strip is far (zoomed); out, from the camera
      # the land two pixels to the left and to the right, for CARDIAC to work out how it faces the sun
      land(x - 2 * u, di); hl = int(h + 0.5); land(x + 2 * u, di); hr = int(h + 0.5)
      land(x, di)
      horizon = int(horizon0 + (col - 48) * bank / 96)
      printf " %d %d %d %d", horizon, 10 * int(h + 0.5) + c, hl, hr > deck
    }
    printf "\n" > deck
    close(deck)
  }
}'

# CARDIAC works out every slice
s=1
while [ $s -le "$SLICES" ]; do
  "$BURN" voxel.cqb -- $(cat "$work/slice$s") > "$work/out$s" 2>/dev/null
  s=$((s + 1))
done

# The host puts CARDIAC's answers on the screen, near to far: in each column, a slice's land shows from the row
# CARDIAC gave down to the top of what is already there, in the colour CARDIAC gave, 10 c plus a light, looked up in
# a fixed table, as a display's palette is; the rest is sky.
awk -v slices="$SLICES" -v work="$work" -v palette="$PALETTE" 'BEGIN {
  if (palette == "desert") split("40 90 170|150 64 38|185 92 52|204 128 78|214 172 122|118 128 78", pal, "|")
  else if (palette == "mountain") split("40 90 170|40 84 44|56 110 54|120 150 80|128 118 108|240 240 245", pal, "|")
  else split("40 90 170|200 190 120|90 160 70|60 120 50|125 110 100|240 240 245", pal, "|")
  split("0.6 0.8 1 1.15 1.3", lit, " ")
  for (c = 0; c < 6; c++) {
    split(pal[c + 1], rgb, " ")
    for (l = 0; l < 5; l++) for (k = 1; k <= 3; k++) { v = int(rgb[k] * lit[l + 1]); TABLE[10 * c + l, k] = v > 255 ? 255 : v }
  }
  for (col = 0; col < 96; col++) top[col] = 96
  for (s = 1; s <= slices; s++) {
    f = work "/out" s; col = 0
    while ((getline row < f) > 0) {
      getline code < f
      for (r = row; r < top[col]; r++) { R[r, col] = TABLE[code, 1]; G[r, col] = TABLE[code, 2]; B[r, col] = TABLE[code, 3] }
      if (row < top[col]) top[col] = row
      col++
    }
    close(f)
  }
  for (r = 0; r < 96; r++) for (col = 0; col < 96; col++)
    if (r < top[col]) { R[r, col] = 120; G[r, col] = 170; B[r, col] = 230 }
  print "P3\n96 96\n255" > "voxel.ppm"
  for (r = 0; r < 96; r++)
    for (col = 0; col < 96; col++) print R[r, col], G[r, col], B[r, col] > "voxel.ppm"
}'
# And the map: MAP itself, the camera (red) at the top looking down it, and every tenth strip (white): the
# 96 points under the screen's columns, a strip wider the farther out.
awk -v slices="$SLICES" -v camx="$CAMX" -v camy="$CAMY" -v map="$MAP" -v look="$LOOK" -v above="$ABOVE" -v camz="$CAMZ" -v water_below="$WATER" -v palette="$PALETTE" -v unit="$UNIT" -v flats="$FLAT" -v farpx="$FAR" -v relief="$RELIEF" -v zoom="$ZOOM" -v bank="$BANK" -v horizon0="$HORIZON" -v nearpx="$NEAR" "$LAND"' BEGIN {
  load(); N = 256
  for (r = 0; r < N; r++) for (q = 0; q < N; q++) { v = int(px(r, q)); R[r, q] = v; G[r, q] = v; B[r, q] = v }
  far = farpx * u; near = nearpx * u
  for (s = 1; s <= slices; s++) {
    di = int(near * (far / near) ^ ((s - 1) / (slices - 1)) + 0.5)
    if (s % 10 == 0) for (col = 0; col < 96; col++) {
      x = (col - 48) * di / 48 / zoom
      q = int(camx + (x * rx + di * fx) / u); r = int(camy + (x * ry + di * fy) / u)
      if (q >= 0 && q < N && r >= 0 && r < N) { R[r, q] = 255; G[r, q] = 255; B[r, q] = 255 }
    }
  }
  for (r = camy - 2; r <= camy + 2; r++) for (q = camx - 2; q <= camx + 2; q++) { R[r, q] = 230; G[r, q] = 40; B[r, q] = 40 }
  print "P3\n" N " " N "\n255" > "voxel_map.ppm"
  for (r = 0; r < N; r++) for (q = 0; q < N; q++) print R[r, q], G[r, q], B[r, q] > "voxel_map.ppm"
}'
rm -f voxel.png voxel_map.png
if command -v sips >/dev/null; then
  sips -s format png voxel.ppm --out voxel.png >/dev/null 2>&1 && sips -z 384 384 voxel.png >/dev/null 2>&1 || true
  sips -s format png voxel_map.ppm --out voxel_map.png >/dev/null 2>&1 && sips -z 512 512 voxel_map.png >/dev/null 2>&1 || true
fi
if [ -f voxel.png ]; then echo "voxel.sh: wrote voxel.ppm, voxel_map.ppm and their .png ($SLICES slices on CARDIAC)"
else echo "voxel.sh: wrote voxel.ppm and voxel_map.ppm ($SLICES slices on CARDIAC)"; fi
