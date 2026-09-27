;; Never returns from its first tick.
(module
  (memory (export "memory") 1)
  (func (export "update")
    (loop $forever (br $forever))))
