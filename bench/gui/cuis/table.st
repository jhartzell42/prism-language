"The GUI benchmark's table in Cuis Smalltalk with Morphic, on the Cog VM:
the same rows, labels, buttons and operations as bench/prism/bench.prism, and its own
driver. The rows are in a scroll pane in a 900 by 800 window, as GTK's are in a scrolled
window and the Prism window clips them. A step performs its button's action, as a click would, then forces a World cycle
(doOneCycleNow: layout, drawing, and the damage put on the Display); a timed step's clock
runs from just before the action to the end of that cycle. bench/gui/run.sh writes OP, RUNS
and WARMUP in for the placeholders and runs this with -s; it prints `OP RUN MS` per timed
run on standard output, then quits.

Written by Claude on the claude-self-hosting branch; subject to change."
[| out op runs warmup adjectives colours nouns seed nextSeed pick nextId rows list selected showLabel addRow clearRows
   create append update clear swap select remove top window scroller buttons steps scripts world t0 run |
out := StdIOWriteStream stdout.
op := '@OP@'. runs := @RUNS@. warmup := @WARMUP@.
adjectives := #('pretty' 'large' 'big' 'small' 'tall' 'short' 'long' 'handsome' 'plain' 'quaint' 'clean' 'elegant' 'easy' 'angry' 'crazy' 'helpful' 'mushy' 'odd' 'unsightly' 'adorable' 'important' 'inexpensive' 'cheap' 'expensive' 'fancy').
colours := #('red' 'yellow' 'blue' 'green' 'pink' 'brown' 'purple' 'brown' 'white' 'black' 'orange').
nouns := #('table' 'chair' 'house' 'bbq' 'desk' 'car' 'pony' 'cookie' 'sandwich' 'burger' 'pizza' 'mouse' 'keyboard').
"bench.prism's generator, so all three make the same labels"
seed := 42.
nextSeed := [:s | (s * 1103515245 + 12345) \\ 2147483648].
pick := [:words :s | words at: (s // 65536) \\ words size + 1].
nextId := 1.
"each row: {id. label. rowMorph. labelButton}"
rows := OrderedCollection new.
selected := nil.
list := LinearLayoutMorph newColumn separation: 2.
showLabel := [:r |
	(r at: 4) label: (r == selected ifTrue: [(UnicodeString with: (Character codePoint: 16r25B6)), ' ', (r at: 2)] ifFalse: [r at: 2])].
addRow := [| s1 s2 s3 r rowMorph labelButton |
	s1 := nextSeed value: seed. s2 := nextSeed value: s1. s3 := nextSeed value: s2. seed := s3.
	r := Array new: 4.
	r at: 1 put: nextId. nextId := nextId + 1.
	r at: 2 put: (pick value: adjectives value: s1), ' ', (pick value: colours value: s2), ' ', (pick value: nouns value: s3).
	rowMorph := LinearLayoutMorph newRow separation: 4.
	labelButton := PluggableButtonMorph model: [select value: r] action: #value label: (r at: 2).
	rowMorph
		addMorphBack: (LabelMorph contents: (r at: 1) printString);
		addMorphBack: labelButton;
		addMorphBack: (PluggableButtonMorph model: [remove value: r] action: #value label: 'x').
	r at: 3 put: rowMorph. r at: 4 put: labelButton.
	list addMorphBack: rowMorph.
	rows addLast: r].
clearRows := [rows do: [:r | (r at: 3) delete]. rows := OrderedCollection new. selected := nil].
create := [:n | clearRows value. n timesRepeat: [addRow value]].
append := [:n | n timesRepeat: [addRow value]].
update := [1 to: rows size by: 10 do: [:i | | r | r := rows at: i. r at: 2 put: (r at: 2), ' !!!'. showLabel value: r]].
clear := [clearRows value].
swap := [rows size >= 999 ifTrue: [| a z |
	a := rows at: 2. z := rows at: 999.
	rows at: 2 put: z. rows at: 999 put: a.
	"Morphic lays the whole column out again on any change, so the two rows are moved
	by putting the column's rows back in their new order."
	list removeAllMorphs.
	rows do: [:r | list addMorphBack: (r at: 3)]]].
select := [:r | | old | old := selected. selected := r. old ifNotNil: [showLabel value: old]. showLabel value: r].
remove := [:r | rows remove: r. r == selected ifTrue: [selected := nil]. (r at: 3) delete].

window := LinearLayoutMorph newColumn separation: 2.
top := LinearLayoutMorph newRow separation: 4.
buttons := Dictionary new.
{ 'Create 1,000 rows' -> [create value: 1000]. 'Create 10,000 rows' -> [create value: 10000].
  'Append 1,000 rows' -> [append value: 1000]. 'Update every 10th row' -> update. 'Clear' -> clear. 'Swap rows' -> swap }
	do: [:each | buttons at: each key put: each value.
		top addMorphBack: (PluggableButtonMorph model: each value action: #value label: each key)].
scroller := PluggableScrollPane new.
scroller scroller: list.
window addMorphBack: top; addMorphBack: scroller.
scroller layoutSpec: LayoutSizeSpec useAll.
window position: 0@0 extent: 900@800.
window openInWorld.

"each operation's script: the steps' actions, the timed one marked"
scripts := Dictionary new.
scripts
	at: 'create1k' put: { {'Create 1,000 rows'. true}. {'Clear'. false} };
	at: 'replace1k' put: { {'Create 1,000 rows'. false}. {'Create 1,000 rows'. true}. {'Clear'. false} };
	at: 'update10th' put: { {'Create 1,000 rows'. false}. {'Update every 10th row'. true}. {'Clear'. false} };
	at: 'select' put: { {'Create 1,000 rows'. false}. {#rowLabel. true}. {'Clear'. false} };
	at: 'swap' put: { {'Create 1,000 rows'. false}. {'Swap rows'. true}. {'Clear'. false} };
	at: 'remove' put: { {'Create 1,000 rows'. false}. {#rowX. true}. {'Clear'. false} };
	at: 'create10k' put: { {'Create 10,000 rows'. true}. {'Clear'. false} };
	at: 'append1k' put: { {'Create 1,000 rows'. false}. {'Append 1,000 rows'. true}. {'Clear'. false} };
	at: 'clear' put: { {'Create 1,000 rows'. false}. {'Clear'. true} }.
steps := scripts at: op.
world := window world.
world doOneCycleNow.
run := 0.
1 to: warmup + runs do: [:i |
	steps do: [:s | | action |
		action := s first = #rowLabel
			ifTrue: [[select value: (rows at: 2)]]
			ifFalse: [s first = #rowX ifTrue: [[remove value: (rows at: 2)]] ifFalse: [buttons at: s first]].
		t0 := Time localMicrosecondClock.
		action value.
		world doOneCycleNow.
		(s second and: [i > warmup]) ifTrue: [
			run := run + 1.
			out nextPutAll: op, ' ', run printString, ' ',
				(((Time localMicrosecondClock - t0) / 1000.0) roundTo: 0.001) printString; newLine; flush]]].
Smalltalk quitPrimitive ] on: Error do: [:e |
	StdIOWriteStream stdout nextPutAll: 'cuis: ', e class name, ': ', e messageText asString; newLine; flush.
	Smalltalk quitPrimitive: 1 ]
