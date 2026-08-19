use std::env;

fn main() {
    let cargo_manifest_dir = env!("CARGO_MANIFEST_DIR");
    let secure = if env::var("CARGO_FEATURE_SECURE").is_ok() {
        Some("secure")
    } else {
        None
    };
    let extended = if env::var("CARGO_FEATURE_EXTENDED").is_ok() {
        Some("extended")
    } else {
        None
    };
    let version = if env::var("CARGO_FEATURE_V2").is_ok() {
        "v2"
    } else {
        "v3"
    };

    let mut cfg = ctest::TestGenerator::new();
    cfg.header("mimalloc.h")
        .include(format!(
            "{cargo_manifest_dir}/../c_src/mimalloc/{version}/include"
        ))
        .cfg("feature", secure)
        .cfg("feature", extended)
        .cfg("feature", (version == "v2").then_some("v2"))
        .edition(2024)
        .rename_fn(|function| function.link_name().map(str::to_owned))
        // ignore whether or not the option enum is signed.
        .skip_signededness(|c| c.ends_with("_t") || c.ends_with("_e"))
        .rename_type(|ty| {
            Some(match ty {
                // Special cases. We do this to avoid having both
                // `mi_blah_{s,e}` and `mi_blah_t`.
                "mi_heap_area_t" => "struct mi_heap_area_s".into(),
                "mi_heap_t" => "struct mi_heap_s".into(),
                "mi_options_t" => "enum mi_options_e".into(),

                // This also works but requires we export `mi_heap_s` and similar
                // in addition, so we just hardcode the above.

                // t if t.ends_with("_s") => format!("struct {}", t),
                // t if t.ends_with("_e") => format!("enum {}", t),
                // t if t.ends_with("_t") => t.to_string(),

                // mimalloc defines it's callbacks with the pointer at the
                // location of use, e.g. `typedef ret mi_some_fun(a0 x, a1 y);`
                // and then uses `mi_some_fun *arg` as argument types, which
                // appears to upset ctest, which would prefer function pointers
                // be declared as pointers, so we clean things up for it.
                t if t.ends_with("_fun") => format!("{}*", t),

                _ => return None,
            })
        })
        .rename_struct_ty(|ty| (ty == "mi_heap_area_t").then(|| "struct mi_heap_area_s".into()));

    if version == "v3" {
        cfg.header("mimalloc-stats.h").include(format!(
            "{cargo_manifest_dir}/../c_src/mimalloc/{version}/include"
        ));
    }

    ctest::generate_test(&mut cfg, "../src/lib.rs", "all.rs").unwrap();
}
