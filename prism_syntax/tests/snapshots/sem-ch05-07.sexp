(module
  (sig Components (fn Nat [(tuple Nat Nat)] -> [Nat]))
  (fn Components (params n edges)
    (update (array)
      (let &parent (call Buffer.Numbered n))
      (for (tuple a b) <- edges
        (let &ra a)
        (while (<- (free? p) (index &parent &ra) (if (!= p &ra)))
          (set &ra p))
        (let &rb b)
        (while (<- (free? p) (index &parent &rb) (if (!= p &rb)))
          (set &rb p))
        (set (index &parent &ra) (free? &rb)))
      (set &_ (call Freeze &parent)))))
