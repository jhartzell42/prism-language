(module
  (name Counter)
  (emit (row Ui.*))
  (use Ui)
  (type Step choice
    (case Up)
    (case Down))
  (derive ^Eq ^Show)
  (sig Delta (fn Step -> Int))
  (fn Delta (params Up)
    (pos 1))
  (fn Delta (params Down)
    (neg 1))
  (let start 0)
  (sig @counter @unit)
  (widget @counter
    (bind !inc (call() Ui.Button label=$"Increment"))
    (bind !dec (call() Ui.Button label=$"Decrement"))
    (hold $count start then (+ $count (call Delta (group (<| (on Up !inc) (on Down !dec))))))
    (call() Ui.Label text=($text "Count: " {$count})))
  (check delta_cancels
    (line (== (+ (group (call Delta Up)) (group (call Delta Down))) 0))))
