;; Crashes on its first tick.
(module
  (memory (export "memory") 1)
  (func (export "update")
    unreachable))
