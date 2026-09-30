;; Publishes every kind of widget the runtime has, with every kind of file
;; filter, then stores values of every type in its settings map itself, as
;; `asr`'s `Map::store` does. It runs once, on the first tick:
;;
;; - headings at levels 0 to 3, one with a tooltip;
;; - a checkbox "gym" (default on, with a tooltip);
;; - a choice "category" (Any% or 100%, default Any%);
;; - a text input "level" (default "prologue");
;; - a file selection "route" with a named filter of two patterns, an unnamed
;;   one, a glob that isn't an extension, and two MIME type filters;
;; - then it stores int, float, list and map values under keys that aren't
;;   widgets, and "level" = "chapter_2", a widget's key, keeping everything
;;   else in the map.
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
    (func $name_filter (param i32 i32 i32 i32 i32 i32)))
  (import "env" "user_settings_add_file_select_mime_filter"
    (func $mime_filter (param i32 i32 i32 i32)))
  (import "env" "user_settings_add_text_input"
    (func $text (param i32 i32 i32 i32 i32 i32)))
  (import "env" "user_settings_set_tooltip"
    (func $tooltip (param i32 i32 i32 i32)))
  (import "env" "settings_map_load" (func $map_load (result i64)))
  (import "env" "settings_map_new" (func $map_new (result i64)))
  (import "env" "settings_map_insert" (func $map_insert (param i64 i32 i32 i64)))
  (import "env" "settings_map_store" (func $map_store (param i64)))
  (import "env" "settings_map_free" (func $map_free (param i64)))
  (import "env" "settings_list_new" (func $list_new (result i64)))
  (import "env" "settings_list_push" (func $list_push (param i64 i64)))
  (import "env" "settings_list_free" (func $list_free (param i64)))
  (import "env" "setting_value_new_bool" (func $new_bool (param i32) (result i64)))
  (import "env" "setting_value_new_i64" (func $new_i64 (param i64) (result i64)))
  (import "env" "setting_value_new_f64" (func $new_f64 (param f64) (result i64)))
  (import "env" "setting_value_new_string" (func $new_string (param i32 i32) (result i64)))
  (import "env" "setting_value_new_list" (func $new_list (param i64) (result i64)))
  (import "env" "setting_value_new_map" (func $new_map (param i64) (result i64)))
  (memory (export "memory") 1)
  ;; Headings.
  (data (i32.const 0) "general")
  (data (i32.const 16) "General")
  (data (i32.const 32) "files")
  (data (i32.const 48) "Files")
  (data (i32.const 64) "collectibles")
  (data (i32.const 80) "Collectibles")
  (data (i32.const 96) "horseshoes")
  (data (i32.const 112) "Horseshoes by area")
  (data (i32.const 144) "Options used during normal operation.")
  ;; The checkbox.
  (data (i32.const 192) "gym")
  (data (i32.const 208) "Gym Moves")
  (data (i32.const 224) "Split when the mission is passed.")
  ;; The choice.
  (data (i32.const 272) "category")
  (data (i32.const 288) "Category")
  (data (i32.const 304) "any")
  (data (i32.const 320) "Any%")
  (data (i32.const 336) "hundred")
  (data (i32.const 352) "100%")
  ;; The text input.
  (data (i32.const 368) "level")
  (data (i32.const 384) "Split on level")
  (data (i32.const 400) "prologue")
  ;; The file selection and its filters.
  (data (i32.const 416) "route")
  (data (i32.const 432) "Route")
  (data (i32.const 448) "Images")
  (data (i32.const 464) "*.png *.jpg")
  (data (i32.const 480) "*.json")
  (data (i32.const 496) "Rust")
  (data (i32.const 512) "*.rs Cargo.*")
  (data (i32.const 528) "image/*")
  (data (i32.const 544) "text/plain")
  ;; Stored values.
  (data (i32.const 576) "best_route_version")
  (data (i32.const 608) "igt_offset")
  (data (i32.const 624) "completed_missions")
  (data (i32.const 656) "intro")
  (data (i32.const 672) "last_run")
  (data (i32.const 688) "splits")
  (data (i32.const 704) "chapter_2")
  (global $done (mut i32) (i32.const 0))
  (func (export "update")
    (local $map i64)
    (local $inner i64)
    (local $list i64)
    (if (global.get $done) (then (return)))
    (global.set $done (i32.const 1))

    (call $title (i32.const 0) (i32.const 7) (i32.const 16) (i32.const 7) (i32.const 0))
    (call $tooltip (i32.const 0) (i32.const 7) (i32.const 144) (i32.const 37))
    (drop (call $bool (i32.const 192) (i32.const 3) (i32.const 208) (i32.const 9) (i32.const 1)))
    (call $tooltip (i32.const 192) (i32.const 3) (i32.const 224) (i32.const 33))
    (call $choice (i32.const 272) (i32.const 8) (i32.const 288) (i32.const 8) (i32.const 304) (i32.const 3))
    (drop (call $option (i32.const 272) (i32.const 8) (i32.const 304) (i32.const 3) (i32.const 320) (i32.const 4)))
    (drop (call $option (i32.const 272) (i32.const 8) (i32.const 336) (i32.const 7) (i32.const 352) (i32.const 4)))
    (call $text (i32.const 368) (i32.const 5) (i32.const 384) (i32.const 14) (i32.const 400) (i32.const 8))
    (call $title (i32.const 32) (i32.const 5) (i32.const 48) (i32.const 5) (i32.const 1))
    (call $file (i32.const 416) (i32.const 5) (i32.const 432) (i32.const 5))
    (call $name_filter (i32.const 416) (i32.const 5) (i32.const 448) (i32.const 6) (i32.const 464) (i32.const 11))
    (call $name_filter (i32.const 416) (i32.const 5) (i32.const 0) (i32.const 0) (i32.const 480) (i32.const 6))
    (call $name_filter (i32.const 416) (i32.const 5) (i32.const 496) (i32.const 4) (i32.const 512) (i32.const 12))
    (call $mime_filter (i32.const 416) (i32.const 5) (i32.const 528) (i32.const 7))
    (call $mime_filter (i32.const 416) (i32.const 5) (i32.const 544) (i32.const 10))
    (call $title (i32.const 64) (i32.const 12) (i32.const 80) (i32.const 12) (i32.const 2))
    (call $title (i32.const 96) (i32.const 10) (i32.const 112) (i32.const 18) (i32.const 3))

    ;; The map as it is, with the values added.
    (local.set $map (call $map_load))
    (call $map_insert (local.get $map) (i32.const 576) (i32.const 18) (call $new_i64 (i64.const 7)))
    (call $map_insert (local.get $map) (i32.const 608) (i32.const 10) (call $new_f64 (f64.const 1.25)))
    (local.set $list (call $list_new))
    (call $list_push (local.get $list) (call $new_string (i32.const 656) (i32.const 5)))
    (call $list_push (local.get $list) (call $new_bool (i32.const 1)))
    (call $map_insert (local.get $map) (i32.const 624) (i32.const 18) (call $new_list (local.get $list)))
    (call $list_free (local.get $list))
    (local.set $inner (call $map_new))
    (call $map_insert (local.get $inner) (i32.const 688) (i32.const 6) (call $new_i64 (i64.const 42)))
    (call $map_insert (local.get $map) (i32.const 672) (i32.const 8) (call $new_map (local.get $inner)))
    (call $map_free (local.get $inner))
    (call $map_insert (local.get $map) (i32.const 368) (i32.const 5) (call $new_string (i32.const 704) (i32.const 9)))
    (call $map_store (local.get $map))
    (call $map_free (local.get $map))))
