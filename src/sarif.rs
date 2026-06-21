//! SARIF 2.1.0 output with real `file:line` locations, for GitHub code scanning.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::finding::Report;
use crate::rules::{Level, RULES};

const INFO_URI: &str = "https://github.com/studiomeyer-io/mcp-herald";

fn level_str(l: Level) -> &'static str {
    match l {
        Level::Error => "error",
        Level::Warning => "warning",
        Level::Info => "note",
    }
}

/// Render a scan report as SARIF. Only the rules that actually fired are emitted as rule
/// descriptors, each with its SEP source as `helpUri`.
pub fn to_sarif(report: &Report) -> Value {
    // Distinct fired rules -> descriptors (with helpUri + short description).
    let mut fired: BTreeMap<&str, &'static str> = BTreeMap::new();
    for f in &report.findings {
        fired.insert(f.rule, f.url);
    }
    let rules: Vec<Value> = fired
        .keys()
        .map(|id| {
            let spec = RULES.iter().find(|r| r.id == *id);
            json!({
                "id": id,
                "name": id,
                "shortDescription": { "text": spec.map(|s| s.message).unwrap_or("") },
                "helpUri": spec.map(|s| s.url).unwrap_or(INFO_URI),
                "defaultConfiguration": {
                    "level": spec.map(|s| level_str(s.level)).unwrap_or("warning")
                }
            })
        })
        .collect();

    let results: Vec<Value> = report
        .findings
        .iter()
        .map(|f| {
            json!({
                "ruleId": f.rule,
                "level": level_str(f.level),
                "message": { "text": format!("{} [{}] Fix: {}", f.message, f.sep, f.fix) },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": f.file },
                        "region": { "startLine": f.line.max(1) }
                    }
                }]
            })
        })
        .collect();

    json!({
        "version": "2.1.0",
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "mcp-herald",
                    "informationUri": INFO_URI,
                    "version": env!("CARGO_PKG_VERSION"),
                    "rules": rules
                }
            },
            "results": results
        }]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::compile;
    use crate::scanner::scan_content;

    #[test]
    fn sarif_has_real_line_locations() {
        let rules = compile();
        let mut findings = Vec::new();
        scan_content(
            "src/server.ts",
            "x\nthrow new McpError(-32002);",
            &rules,
            &mut findings,
        );
        let report = crate::finding::Report { findings };
        let s = to_sarif(&report);
        assert_eq!(s["version"], "2.1.0");
        let res = &s["runs"][0]["results"][0];
        assert_eq!(res["ruleId"], "spec.error_code");
        assert_eq!(res["level"], "error");
        assert_eq!(
            res["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            "src/server.ts"
        );
        assert_eq!(
            res["locations"][0]["physicalLocation"]["region"]["startLine"],
            2
        );
    }
}
