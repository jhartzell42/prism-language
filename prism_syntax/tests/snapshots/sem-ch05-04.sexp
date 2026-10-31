(module
  (sig Step (fn Vm Op -> Vm))
  (fn Step (params vm op)
    (update vm
      (+= (. &_ steps) 1)
      (match op
        (arm (ctor Add n)
          (+= (. (. &_ regs) acc) n)
          (+= (. &_ pc) 1))
        (arm Swap
          (let a (. (. &_ regs) acc))
          (set (. (. &_ regs) acc) (. (. &_ regs) x))
          (set (. (. &_ regs) x) a)
          (+= (. &_ pc) 1))
        (arm (ctor JumpIfNeg target)
          (if (< (. (. &_ regs) acc) 0)
            (set (. &_ pc) target)
           else
            (+= (. &_ pc) 1)))))))
