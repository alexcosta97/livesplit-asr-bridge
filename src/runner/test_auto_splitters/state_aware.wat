;; Behaves like a real auto splitter: it checks the timer state first. It
;; starts when the timer isn't running, splits while it is until three
;; segments are done, then resets.
(module
  (import "env" "timer_get_state" (func $state (result i32)))
  (import "env" "timer_current_split_index" (func $split_index (result i64)))
  (import "env" "timer_start" (func $start))
  (import "env" "timer_split" (func $split))
  (import "env" "timer_reset" (func $reset))
  (memory (export "memory") 1)
  (global $done (mut i32) (i32.const 0))
  (func (export "update")
    (if (global.get $done) (then (return)))
    ;; 0: not running
    (if (i32.eqz (call $state))
      (then (call $start) (return)))
    ;; 1: running
    (if (i32.eq (call $state) (i32.const 1))
      (then
        (if (i64.lt_s (call $split_index) (i64.const 3))
          (then (call $split))
          (else
            (call $reset)
            (global.set $done (i32.const 1))))))))
