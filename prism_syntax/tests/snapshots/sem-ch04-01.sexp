(module
  (emit (row Ui.*))
  (type Question record
    (field prompt Text)
    (field truth Bool))
  (let owls (call() Question prompt="Owls are birds" truth=True))
  (let bats (call() Question prompt="Bats are birds" truth=False))
  (let questions (map (1: owls) (2: bats)))
  (sig ?Score (fn (group (==> Nat Bool)) -> ?Nat))
  (fn ?Score (params marks)
    (require (== (group (call Size marks)) (group (call Size questions))))
    (free? (call Len (group (call Filter (group (call Values marks)) (lambda (params right) right))))))
  (sig @quiz @unit)
  (widget @quiz
    (bind !again (call() Ui.Button label=$"Start over"))
    (hold $round 1 then (on (+ $round 1) !again))
    (bind $!rounds (rebuild $round
      (call() Ui.Label text=($text "Round " {_}))
      (for id, q <- questions
        (call() Ui.Label text=(free$ (. q prompt)))
        (bind !yes (call() Ui.Button label=$"True"))
        (bind !no (call() Ui.Button label=$"False"))
        (free@ (<| (on (. q truth) !yes) (on (group (not (. q truth))) !no))))))
    (bind !marks (call @Switch $!rounds))
    (hold $answers (map) then (<| (on (map) !again) (group (call Union !marks $answers))))
    (let $verdict (match (call ?Score $answers)
      (arm (free? n) (inline (text {n} " right")))
      (arm nil (inline "keep going"))))
    (call() Ui.Label text=$verdict)))
