;; Asks for 10 ticks per second and logs "tick" on every tick.
(module
  (import "env" "runtime_print_message" (func $print (param i32 i32)))
  (import "env" "runtime_set_tick_rate" (func $set_tick_rate (param f64)))
  (memory (export "memory") 1)
  (data (i32.const 0) "tick")
  (func (export "update")
    (call $set_tick_rate (f64.const 10))
    (call $print (i32.const 0) (i32.const 4))))
