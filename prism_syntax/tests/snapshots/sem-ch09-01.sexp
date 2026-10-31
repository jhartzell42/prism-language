(module
  (name Offline)
  (emit (row (var r)))
  (use Net)
  (sig @Offline (fn (emit @a (| Net.* r)) -> @a))
  (fn @Offline (params @child)
    (bind (tuple x ignore) (call @HostDocument @child (lambda (params p e) (match e
      (arm (record Net.Http ..) (inline (free@ never)))
      (arm (record Net.Socket ..)
        (free@ (tuple never (free$ (group (call Disconnected "offline"))))))
      (arm otherwise (inline (call @Element e)))))))
    (free@ x)))
