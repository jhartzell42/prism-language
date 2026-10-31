(module
  (trait ^Timeline f
    (sig !Updates (fn (f a) -> !a))
    (sig @Following (fn (f a) !a -> (free@ (group (f a))))))
  (impl ^Timeline !_
    (fn !Updates (params !e)
      !e)
    (fn @Following (params !e !later)
      (free@ !later)))
  (impl ^Timeline $_
    (fn !Updates (params $d)
      (call !Updated $d))
    (fn @Following (params $d !later)
      (hold $$current $d then (free$ !later))
      (call @Flatten $$current)))
  (sig @Debounce (fn Duration (f a) -> (free@ (group (f a)))) (where f ^Timeline))
  (fn @Debounce (params d x)
    (bind !quiet (call @Debounced d (group (call !Updates x))))
    (call @Following x !quiet)))
