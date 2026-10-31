(module
  (emit (row Cards.Element))
  (widget @$?firstCard
    (bind !started @started)
    (bind !card (call() Cards.Deck hand=1 draws=(group (on unit !started))))
    (hold $?shown nil then (free? !card))
    (free@ $?shown))
  (test seed_seven_deals_the_four_of_spades @$?firstCard (seed 7)
    (step build
      (=> returns nil))
    (step tick
      (=> requested (record Cards.Deck hand=1) unit))
    (step (answer (record Cards.Deck hand=1))
      (=> returns (free? (record Card rank=Four suit=Spades))))))
