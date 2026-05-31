use super::support::*;

#[test]
fn bridge_sources_do_not_hardcode_domain_modules() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let roots = [manifest_dir.join("src"), manifest_dir.join("../core/src")];
    let forbidden = [
        concat!("std", ".", "character"),
        concat!("moveset", "_", "patcher"),
        concat!("struct", "_", "api"),
        concat!("character", "_", "extension"),
    ];

    for root in roots {
        for source_file in rust_sources_under(&root) {
            if source_file
                .components()
                .any(|component| component.as_os_str() == "tests")
                || source_file.ends_with("tests.rs")
            {
                continue;
            }
            let source = fs::read_to_string(&source_file).expect("source file");
            for token in forbidden {
                assert!(
                    !source.contains(token),
                    "domain token {token:?} found in {}",
                    source_file.display()
                );
            }
        }
    }
}
