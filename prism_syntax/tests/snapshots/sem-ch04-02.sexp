(module
  (sig @Booking (fn $Room $Nat -> @!Nat))
  (fn @Booking (params $room $nights)
    (bind $!asks (rebuild $room
      (call() Ui.Label text=($text {(. _ name)} ", " {(. _ beds)} " beds, " {$nights} " nights"))
      (bind !more (call() Ui.Button label=$"Another guest"))
      (hold $guests 1 then (on (+ $guests 1) !more))
      (free@ (on $guests !more))))
    (call @Switch $!asks)))
