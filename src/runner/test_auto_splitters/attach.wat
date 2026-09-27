;; Attaches to the process whose id replaces PID on its first tick, and
;; detaches on its third.
(module
  (import "env" "process_attach_by_pid" (func $attach (param i64) (result i64)))
  (import "env" "process_detach" (func $detach (param i64)))
  (memory (export "memory") 1)
  (global $tick (mut i32) (i32.const 0))
  (global $process (mut i64) (i64.const 0))
  (func (export "update")
    (global.set $tick (i32.add (global.get $tick) (i32.const 1)))
    (block $done
      (br_if $done (i32.ne (global.get $tick) (i32.const 1)))
      (global.set $process (call $attach (i64.const PID))))
    (block $done
      (br_if $done (i32.ne (global.get $tick) (i32.const 3)))
      (br_if $done (i64.eqz (global.get $process)))
      (call $detach (global.get $process)))))
