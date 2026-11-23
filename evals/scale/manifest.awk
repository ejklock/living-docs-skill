#!/usr/bin/awk -f
#
# manifest.awk — the ADR sequence of one scale scenario, from a seed. Inputs: concerns.tsv then
# components.txt; variables seed, size, plant_pos (0 for none) and plant (title, description,
# context and decision joined by tabs). Prints one line per operation, tab-separated:
#   adr NUMBER TITLE DESCRIPTION CONTEXT DECISION   (in number order, every one Accepted)
#   supersede OLD NEW
#   deprecate NUMBER
# The generator is a Park-Miller sequence, exact in double precision, so every awk prints the
# same manifest for the same inputs.

BEGIN { FS = "\t"; state = (seed % 2147483646) + 1 }

function next_below(n) {
	state = (state * 16807) % 2147483647
	return state % n
}

function shuffle(arr, n,    i, j, tmp) {
	for (i = n - 1; i > 0; i--) {
		j = next_below(i + 1)
		tmp = arr[i]; arr[i] = arr[j]; arr[j] = tmp
	}
}

function fail(message) {
	print "manifest: " message > "/dev/stderr"
	failed = 1
	exit 1
}

function filler(i, variant,    parts, c, k, row, name, short, sentence, context) {
	split(topic[i], parts, ",")
	c = parts[1]; k = parts[2]
	split(concern[k], row, "\t")
	name = component[c]
	short = (variant == "old") ? row[4] : row[2]
	sentence = (variant == "old") ? row[5] : row[3]
	context = (variant == "new") ? "The " name " service outgrew its earlier choice on " row[1] "." : "The " name " service needed one agreed answer on " row[1] "."
	return toupper(substr(name, 1, 1)) substr(name, 2) " service: " short "\tThe " name " service will " sentence ".\t" context "\t" sentence " in the " name " service"
}

function build_topics(    c, k) {
	for (c = 0; c < np; c++)
		for (k = 0; k < nc; k++)
			topic[nt++] = c "," k
	shuffle(topic, nt)
}

function build_entries(    i) {
	for (i = 0; i < topics; i++) entry[ne++] = i
	for (i = 0; i < pairs; i++) entry[ne++] = i
	shuffle(entry, ne)
}

function place(    j, n, i, variant) {
	n = 0
	for (j = 0; j < ne; j++) {
		if (plant_pos > 0 && n + 1 == plant_pos) line[++n] = plant
		i = entry[j]
		variant = (i >= pairs) ? "plain" : ((i in first) ? "new" : "old")
		if (variant == "old") first[i] = n + 1
		if (variant == "new") second[i] = n + 1
		if (i >= pairs) number_of[i] = n + 1
		line[++n] = filler(i, variant)
	}
	if (plant_pos > 0 && n + 1 == plant_pos) line[++n] = plant
	return n
}

FNR == NR { concern[nc++] = $0; next }
NF > 0 { component[np++] = $0 }

END {
	if (failed) exit 1
	pairs = int(size / 8); deprecated = int(size / 12)
	topics = size - (plant_pos > 0) - pairs
	build_topics()
	if (topics > nt) fail("size " size " needs " topics " topics, the table holds " nt)
	if (plant_pos > size || plant_pos < 0) fail("plant position " plant_pos " is outside 1.." size)
	build_entries()
	if (place() != size) fail("built a sequence of the wrong length")
	for (n = 1; n <= size; n++) print "adr\t" n "\t" line[n]
	for (i = 0; i < pairs; i++) print "supersede\t" first[i] "\t" second[i]
	for (i = pairs; i < pairs + deprecated; i++) print "deprecate\t" number_of[i]
}
