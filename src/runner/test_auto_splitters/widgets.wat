;; Publishes one widget of every kind on its first tick: a heading at each
;; level, a checkbox with a tooltip, a choice, a file selection with a
;; filter, and a text input.
(module
  (import "env" "user_settings_add_title"
    (func $title (param i32 i32 i32 i32 i32)))
  (import "env" "user_settings_add_bool"
    (func $bool (param i32 i32 i32 i32 i32) (result i32)))
  (import "env" "user_settings_add_choice"
    (func $choice (param i32 i32 i32 i32 i32 i32)))
  (import "env" "user_settings_add_choice_option"
    (func $option (param i32 i32 i32 i32 i32 i32) (result i32)))
  (import "env" "user_settings_add_file_select"
    (func $file (param i32 i32 i32 i32)))
  (import "env" "user_settings_add_file_select_name_filter"
    (func $filter (param i32 i32 i32 i32 i32 i32)))
  (import "env" "user_settings_add_text_input"
    (func $text (param i32 i32 i32 i32 i32 i32)))
  (import "env" "user_settings_set_tooltip"
    (func $tooltip (param i32 i32 i32 i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "splits")
  (data (i32.const 16) "Splits")
  (data (i32.const 32) "gym")
  (data (i32.const 48) "Gym Moves")
  (data (i32.const 64) "Split when the mission is passed")
  (data (i32.const 112) "extras")
  (data (i32.const 128) "Extras")
  (data (i32.const 144) "category")
  (data (i32.const 160) "Category")
  (data (i32.const 176) "any")
  (data (i32.const 192) "Any%")
  (data (i32.const 208) "route")
  (data (i32.const 224) "Route")
  (data (i32.const 240) "*.txt")
  (data (i32.const 256) "runner")
  (data (i32.const 272) "Runner")
  (data (i32.const 288) "me")
  (global $registered (mut i32) (i32.const 0))
  (func (export "update")
    (if (global.get $registered) (then (return)))
    (global.set $registered (i32.const 1))
    (call $title (i32.const 0) (i32.const 6) (i32.const 16) (i32.const 6) (i32.const 0))
    (drop (call $bool (i32.const 32) (i32.const 3) (i32.const 48) (i32.const 9) (i32.const 1)))
    (call $tooltip (i32.const 32) (i32.const 3) (i32.const 64) (i32.const 32))
    (call $title (i32.const 112) (i32.const 6) (i32.const 128) (i32.const 6) (i32.const 1))
    (call $choice (i32.const 144) (i32.const 8) (i32.const 160) (i32.const 8) (i32.const 176) (i32.const 3))
    (drop (call $option (i32.const 144) (i32.const 8) (i32.const 176) (i32.const 3) (i32.const 192) (i32.const 4)))
    (call $file (i32.const 208) (i32.const 5) (i32.const 224) (i32.const 5))
    (call $filter (i32.const 208) (i32.const 5) (i32.const 0) (i32.const 0) (i32.const 240) (i32.const 5))
    (call $text (i32.const 256) (i32.const 6) (i32.const 272) (i32.const 6) (i32.const 288) (i32.const 2))))
