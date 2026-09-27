;; Starts, splits and logs on a schedule, one step per tick:
;; 1: log "started", start   2: split   3: set game time 1.5 s, split
;; 4: log "finished", reset. After that it does nothing.
(module
  (import "env" "runtime_print_message" (func $print (param i32 i32)))
  (import "env" "timer_start" (func $start))
  (import "env" "timer_split" (func $split))
  (import "env" "timer_reset" (func $reset))
  (import "env" "timer_set_game_time" (func $set_game_time (param i64 i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "started")
  (data (i32.const 16) "finished")
  (global $tick (mut i32) (i32.const 0))
  (func (export "update")
    (global.set $tick (i32.add (global.get $tick) (i32.const 1)))
    (block $done
      (br_if $done (i32.ne (global.get $tick) (i32.const 1)))
      (call $print (i32.const 0) (i32.const 7))
      (call $start))
    (block $done
      (br_if $done (i32.ne (global.get $tick) (i32.const 2)))
      (call $split))
    (block $done
      (br_if $done (i32.ne (global.get $tick) (i32.const 3)))
      (call $set_game_time (i64.const 1) (i32.const 500000000))
      (call $split))
    (block $done
      (br_if $done (i32.ne (global.get $tick) (i32.const 4)))
      (call $print (i32.const 16) (i32.const 8))
      (call $reset))))
