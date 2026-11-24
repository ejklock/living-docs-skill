#!/usr/bin/env bash
#
# plot.sh — draw a forest plot (a rate with its 95% Wilson interval per row) as a standalone SVG.
#
# usage: plot.sh TITLE < ROWS        (ROWS: one line per row, LABEL<TAB>SUCCESSES<TAB>TRIALS)
#
# Each row is a dot at SUCCESSES/TRIALS on a 0 to 1 axis with a whisker over its interval and the
# count printed on the right, in input order. A line with SUCCESSES and TRIALS empty is a group
# heading. The same input always prints the same bytes. Needs bash and awk.
# Every other field count is a malformed row. Exit: 0 = printed, 2 = bad usage, no rows or a malformed row.

set -euo pipefail
export LC_ALL=C

(($# == 1)) && [[ -n "$1" ]] || {
	echo "usage: plot.sh TITLE < ROWS" >&2
	exit 2
}

awk -F'\t' -v title="$1" '
function esc(s) { gsub(/&/, "\\&amp;", s); gsub(/</, "\\&lt;", s); gsub(/>/, "\\&gt;", s); return s }
function x(v) { return left + v * width }
function wilson(k, n,    z, p, d, c, h) {
	z = 1.959963984540054; p = k / n; d = 1 + z * z / n
	c = (p + z * z / (2 * n)) / d
	h = z * sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
	lo = c - h; hi = c + h
	if (lo < 0) lo = 0
	if (hi > 1) hi = 1
}
NF == 0 { next }
{
	if (NF == 3 && $2 == "" && $3 == "") { label[++rows] = $1; heading[rows] = 1; next }
	if (NF != 3 || $2 !~ /^[0-9]+$/ || $3 !~ /^[0-9]+$/ || $3 == 0 || $2 > $3) { print "plot: malformed row " NR ": " $0 > "/dev/stderr"; bad = 1; exit 2 }
	label[++rows] = $1; k[rows] = $2; n[rows] = $3
}
END {
	if (bad) exit 2
	if (rows == 0) { print "plot: no rows" > "/dev/stderr"; exit 2 }
	left = 220; width = 360; top = 48; step = 22; right = left + width
	height = top + rows * step + 44
	printf "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"%d\" height=\"%d\" viewBox=\"0 0 %d %d\" font-family=\"sans-serif\" font-size=\"12\">\n", right + 110, height, right + 110, height
	printf "<rect width=\"100%%\" height=\"100%%\" fill=\"white\"/>\n"
	printf "<text x=\"%d\" y=\"22\" font-size=\"14\" font-weight=\"bold\">%s</text>\n", 10, esc(title)
	axis = top + rows * step + 6
	for (t = 0; t <= 4; t++) {
		v = t / 4
		printf "<line x1=\"%.1f\" y1=\"%d\" x2=\"%.1f\" y2=\"%d\" stroke=\"#ddd\"/>\n", x(v), top - 10, x(v), axis
		printf "<text x=\"%.1f\" y=\"%d\" text-anchor=\"middle\" fill=\"#555\">%.2f</text>\n", x(v), axis + 16, v
	}
	for (i = 1; i <= rows; i++) {
		y = top + (i - 1) * step
		if (heading[i]) { printf "<text x=\"10\" y=\"%d\" font-weight=\"bold\">%s</text>\n", y + 4, esc(label[i]); continue }
		wilson(k[i], n[i]); p = k[i] / n[i]
		printf "<text x=\"%d\" y=\"%d\">%s</text>\n", 24, y + 4, esc(label[i])
		printf "<line x1=\"%.1f\" y1=\"%d\" x2=\"%.1f\" y2=\"%d\" stroke=\"#333\" stroke-width=\"2\"/>\n", x(lo), y, x(hi), y
		printf "<line x1=\"%.1f\" y1=\"%d\" x2=\"%.1f\" y2=\"%d\" stroke=\"#333\"/>\n", x(lo), y - 4, x(lo), y + 4
		printf "<line x1=\"%.1f\" y1=\"%d\" x2=\"%.1f\" y2=\"%d\" stroke=\"#333\"/>\n", x(hi), y - 4, x(hi), y + 4
		printf "<circle cx=\"%.1f\" cy=\"%d\" r=\"4\" fill=\"#1f6feb\"/>\n", x(p), y
		printf "<text x=\"%d\" y=\"%d\" fill=\"#555\">%d/%d [%.2f–%.2f]</text>\n", right + 10, y + 4, k[i], n[i], lo, hi
	}
	printf "<text x=\"%d\" y=\"%d\" fill=\"#555\">rate with 95%% Wilson interval</text>\n", left, axis + 34
	print "</svg>"
}'
