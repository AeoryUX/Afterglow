# GLib 0.18.5 compatibility backport

Afterglow retains the GTK3/WebKitGTK 4.1 API used by Tauri. The registry copy of glib 0.18.5 contains RUSTSEC-2024-0429 (also GHSA-wrw7-89jp-8q8g).

The only source change is in src/variant_iter.rs: make the out-pointer mutable and pass &mut p into g_variant_get_child. This backports the upstream fix shipped in GLib Rust 0.20 without upgrading the incompatible GTK bindings. The original license and sources are retained. The app regression test exercises this iterator with optimized compilation.

Advisory references:
- https://crates.io/crates/glib
- https://rustsec.org/advisories/RUSTSEC-2024-0429.html
- https://github.com/gtk-rs/gtk-rs-core/pull/1343
