//! `scan_path` over a real temp directory: only source extensions are scanned,
//! non-source files (e.g. `.md`) are ignored, and findings come back sorted by
//! (file, line). Uses `std::env::temp_dir()` + a unique subdir and cleans up after.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use mcp_herald::{compile, scan_path};

static SEQ: AtomicU32 = AtomicU32::new(0);

/// RAII temp directory under the OS temp dir — removed on drop even if a test panics.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(tag: &str) -> Self {
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let path = std::env::temp_dir().join(format!("mcp-herald-it-{tag}-{pid}-{n}"));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        TempDir { path }
    }

    fn write(&self, rel: &str, content: &str) {
        let p = self.path.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(p, content).expect("write fixture");
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn scans_only_source_extensions() {
    let dir = TempDir::new("exts");
    // Source files with violations.
    dir.write("server.ts", "throw new McpError(-32002);\n");
    dir.write("handler.py", "raise McpError(-32002)\n");
    // Non-source file that ALSO contains -32002 but must be ignored (no source ext).
    dir.write("README.md", "Historically we used -32002 for not-found.\n");
    dir.write("notes.txt", "another -32002 reference\n");
    dir.write("data.json", r#"{ "code": -32002 }"#);

    let rules = compile();
    let report = scan_path(dir.path(), &rules);

    // Exactly the two source files produced error_code findings.
    let files: Vec<&str> = report
        .findings
        .iter()
        .filter(|f| f.rule == "spec.error_code")
        .map(|f| f.file.as_str())
        .collect();
    assert!(files.contains(&"server.ts"), "ts should be scanned");
    assert!(files.contains(&"handler.py"), "py should be scanned");
    assert!(
        !report.findings.iter().any(|f| f.file.ends_with(".md")),
        ".md must not be scanned"
    );
    assert!(
        !report.findings.iter().any(|f| f.file.ends_with(".txt")),
        ".txt must not be scanned"
    );
    assert!(
        !report.findings.iter().any(|f| f.file.ends_with(".json")),
        ".json must not be scanned"
    );
}

#[test]
fn findings_are_sorted_by_file_then_line() {
    let dir = TempDir::new("sort");
    // Filenames chosen so lexical order is a_, b_, c_.
    dir.write("c_last.ts", "throw e(-32002);\n");
    dir.write("a_first.ts", "// pad\nthrow e(-32002);\n");
    dir.write(
        "b_mid.ts",
        "headers['Mcp-Session-Id'];\n// pad\nthrow e(-32002);\n",
    );

    let rules = compile();
    let report = scan_path(dir.path(), &rules);
    assert!(report.findings.len() >= 4);

    // Verify the (file, line) ordering invariant holds across the whole report.
    for w in report.findings.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        let ord = a.file.cmp(&b.file).then(a.line.cmp(&b.line));
        assert!(
            ord != std::cmp::Ordering::Greater,
            "out of order: {}:{} before {}:{}",
            a.file,
            a.line,
            b.file,
            b.line
        );
    }

    // First file alphabetically is a_first.ts.
    assert_eq!(report.findings.first().unwrap().file, "a_first.ts");
}

#[test]
fn nested_directories_are_walked() {
    let dir = TempDir::new("nested");
    dir.write("src/deep/inner/svc.ts", "throw e(-32002);\n");
    let rules = compile();
    let report = scan_path(dir.path(), &rules);
    assert!(
        report.findings.iter().any(|f| f
            .file
            .replace('\\', "/")
            .ends_with("src/deep/inner/svc.ts")
            && f.rule == "spec.error_code"),
        "nested file should be found, got: {:?}",
        report.findings
    );
}

#[test]
fn clean_tree_yields_empty_report() {
    let dir = TempDir::new("clean");
    dir.write(
        "ok.ts",
        "const t = new Transport({ sessionIdGenerator: undefined });\nreturn -32602;\n",
    );
    let rules = compile();
    let report = scan_path(dir.path(), &rules);
    assert!(
        report.findings.is_empty(),
        "expected clean, got {:?}",
        report.findings
    );
}

#[test]
fn relative_paths_are_reported_relative_to_root() {
    let dir = TempDir::new("relpath");
    dir.write("pkg/server.ts", "throw e(-32002);\n");
    let rules = compile();
    let report = scan_path(dir.path(), &rules);
    let hit = report
        .findings
        .iter()
        .find(|f| f.rule == "spec.error_code")
        .expect("a finding");
    // Path is relative to the scan root, not absolute.
    let normalized = hit.file.replace('\\', "/");
    assert_eq!(normalized, "pkg/server.ts");
    assert!(!Path::new(&hit.file).is_absolute());
}

#[test]
fn single_file_path_is_scannable() {
    let dir = TempDir::new("singlefile");
    dir.write("lonely.ts", "throw e(-32002);\n");
    let rules = compile();
    let file = dir.path().join("lonely.ts");
    let report = scan_path(&file, &rules);
    assert!(report.findings.iter().any(|f| f.rule == "spec.error_code"));
}
