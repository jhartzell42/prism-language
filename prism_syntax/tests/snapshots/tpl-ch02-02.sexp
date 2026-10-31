(module
  (sig @ModuleFile (fn $Module -> @$WriteState))
  (fn @ModuleFile (params $m)
    (emit (row Fs.*))
    (bind $text (call @Serialized (group (call @RustText $m))))
    (call() Fs.Output path=(. $m path) text=$text))
  (sig @RustText (fn $Module -> @unit))
  (fn @RustText (params $m)
    (template Output <rust>
      "// generated from "
      {{(. $m path)}}
      "\n"
      (brace-block
        (for name, $c <- (. $m classes)
          (call @ClassText $c)))))
  (sig @ClassText (fn $Class -> @unit))
  (fn @ClassText (params $c)
    (template Output <rust>
      "pub struct "
      {{(. $c name)}}
      " {\n"
      {{(call Lines (group (call Map (call FieldRs (. $c fields)))))}} @col 4
      "}\n\n"
      (call @VtableText c=$c)
      "\n")))
