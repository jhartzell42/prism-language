(module
  (test a_row_closes_its_file @journal
    (step build
      (=> expect (record Fs.File path="page1.txt")))
    (step (click (at 0)) (click (at 1 #0 0))
      (=> unrequested (record Fs.File ..)))))
