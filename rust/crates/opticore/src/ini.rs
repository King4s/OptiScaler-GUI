//! OptiScaler.ini parsing/serialization with the Python app's semantics:
//! comments accumulate until the next key and drive type inference; writing
//! preserves section order, per-key comments, and creates an .ini.backup.
//! Port of `read_optiscaler_ini` / `write_optiscaler_ini` / `_infer_type`.

use std::collections::BTreeMap;
use std::path::Path;

/// Inferred kind of a setting value, driving the editor widget choice.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueKind {
    /// true / false / auto
    BoolOptions,
    /// Enumerated options from a "0 = A | 1 = B" comment: raw → display label
    Options(BTreeMap<String, String>),
    Int,
    Float,
    Text,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub key: String,
    pub value: String,
    pub comment: String,
    pub kind: ValueKind,
}

#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Default)]
pub struct IniDocument {
    pub sections: Vec<Section>,
}

impl IniDocument {
    pub fn get(&self, section: &str, key: &str) -> Option<&Entry> {
        self.sections
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(section))?
            .entries
            .iter()
            .find(|e| e.key.eq_ignore_ascii_case(key))
    }

    pub fn set_value(&mut self, section: &str, key: &str, value: &str) -> bool {
        if let Some(entry) = self
            .sections
            .iter_mut()
            .find(|s| s.name.eq_ignore_ascii_case(section))
            .and_then(|s| {
                s.entries
                    .iter_mut()
                    .find(|e| e.key.eq_ignore_ascii_case(key))
            })
        {
            entry.value = value.to_string();
            true
        } else {
            false
        }
    }
}

/// Port of `_infer_type`: comment text drives bool/enum detection, then
/// int/float probes, else free text.
pub fn infer_kind(value: &str, comment: &str) -> ValueKind {
    let comment_lower = comment.to_lowercase();
    let value_lower = value.to_lowercase();
    if comment_lower.contains("true or false")
        || (["true", "false", "auto"].contains(&value_lower.as_str())
            && comment_lower.contains("auto"))
    {
        return ValueKind::BoolOptions;
    }

    // "0 = Option A | 1 = Option B" enumerations in the comment
    if let Some(options) = parse_options(comment) {
        let mut options = options;
        if value_lower.contains("auto") || comment_lower.contains("default (auto)") {
            let has_auto = options.values().any(|v| v == "auto");
            if !has_auto {
                options.insert("auto".to_string(), "auto".to_string());
            }
        }
        return ValueKind::Options(options);
    }

    if value.parse::<i64>().is_ok() {
        return ValueKind::Int;
    }
    if value.parse::<f64>().is_ok() {
        return ValueKind::Float;
    }
    ValueKind::Text
}

/// Find a `<digits> = label (| <digits> = label)*` run inside the comment.
fn parse_options(comment: &str) -> Option<BTreeMap<String, String>> {
    // locate the first "<digits> <spaces>? = " occurrence
    let bytes: Vec<char> = comment.chars().collect();
    let mut start = None;
    for i in 0..bytes.len() {
        if bytes[i].is_ascii_digit() {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            let mut k = j;
            while k < bytes.len() && bytes[k] == ' ' {
                k += 1;
            }
            if k < bytes.len() && bytes[k] == '=' {
                start = Some(i);
                break;
            }
        }
    }
    let start = start?;
    let candidate: String = bytes[start..].iter().collect();
    // cut at end of line — options runs don't span comment lines
    let candidate = candidate.lines().next().unwrap_or("");
    let mut options = BTreeMap::new();
    for item in candidate.split('|') {
        if let Some((k, v)) = item.split_once('=') {
            let key = k.trim();
            let value = v.trim();
            if !key.is_empty() && key.chars().all(|c| c.is_ascii_digit()) && !value.is_empty() {
                options.insert(key.to_string(), value.to_string());
            }
        }
    }
    if options.is_empty() {
        None
    } else {
        Some(options)
    }
}

/// Parse an OptiScaler.ini. Same rules as the Python reader: blank lines
/// skipped, `; comments` accumulate until the next key, inline comments
/// stripped from values.
pub fn parse(content: &str) -> IniDocument {
    let mut doc = IniDocument::default();
    let mut current_comments: Vec<String> = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            if let Some(name) = rest.strip_suffix(']') {
                doc.sections.push(Section {
                    name: name.trim().to_string(),
                    entries: Vec::new(),
                });
                current_comments.clear();
            }
        } else if let Some(comment) = line.strip_prefix(';') {
            current_comments.push(comment.trim().to_string());
        } else if let Some((key, value)) = line.split_once('=') {
            let Some(section) = doc.sections.last_mut() else {
                continue; // key-value outside any section
            };
            let value = value.split(';').next().unwrap_or("").trim().to_string();
            let comment = current_comments.join("\n");
            let kind = infer_kind(&value, &comment);
            section.entries.push(Entry {
                key: key.trim().to_string(),
                value,
                comment,
                kind,
            });
            current_comments.clear();
        }
    }
    doc
}

pub fn read_file(ini_path: &Path) -> Option<IniDocument> {
    let content = std::fs::read_to_string(ini_path).ok()?;
    Some(parse(&content))
}

/// Serialize back to INI text (sections, `; comment` lines, key=value).
pub fn serialize(doc: &IniDocument) -> String {
    let mut out = String::new();
    for section in &doc.sections {
        out.push_str(&format!("[{}]\n", section.name));
        for entry in &section.entries {
            for comment_line in entry.comment.split('\n') {
                if !comment_line.trim().is_empty() {
                    out.push_str(&format!("; {comment_line}\n"));
                }
            }
            out.push_str(&format!("{}={}\n", entry.key, entry.value));
        }
        out.push('\n');
    }
    out
}

/// Write with a `.ini.backup` copy of the previous file, like Python.
pub fn write_file(ini_path: &Path, doc: &IniDocument) -> std::io::Result<()> {
    if let Some(parent) = ini_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if ini_path.exists() {
        let backup = ini_path.with_extension("ini.backup");
        std::fs::copy(ini_path, backup)?;
    }
    std::fs::write(ini_path, serialize(doc))
}

/// GPU-vendor-based recommended settings, applied only where the key exists
/// in the document. Port of the settings editor's Auto Settings table.
pub fn auto_settings(vendor: GpuVendor) -> Vec<(&'static str, &'static str, &'static str)> {
    // (section, key, value)
    let mut settings: Vec<(&str, &str, &str)> = match vendor {
        GpuVendor::Nvidia => vec![
            ("Dlss", "Enabled", "true"),
            ("Dlss", "QualityMode", "2"),
            ("Fsr", "Enabled", "auto"),
            ("Fsr", "QualityMode", "2"),
            ("Xess", "Enabled", "auto"),
        ],
        GpuVendor::Amd => vec![
            ("Fsr", "Enabled", "true"),
            ("Fsr", "QualityMode", "2"),
            ("Fsr", "Sharpness", "0.5"),
            ("Dlss", "Enabled", "false"),
            ("Xess", "Enabled", "auto"),
            ("Xess", "QualityMode", "2"),
        ],
        GpuVendor::Intel => vec![
            ("Xess", "Enabled", "true"),
            ("Xess", "QualityMode", "2"),
            ("Fsr", "Enabled", "auto"),
            ("Fsr", "QualityMode", "2"),
            ("Dlss", "Enabled", "false"),
        ],
        GpuVendor::Unknown => vec![
            ("Fsr", "Enabled", "true"),
            ("Fsr", "QualityMode", "3"),
            ("Dlss", "Enabled", "false"),
            ("Xess", "Enabled", "false"),
        ],
    };
    settings.extend([
        ("Performance", "EnableAsyncCompute", "true"),
        ("Performance", "EnableFP16", "true"),
        ("Performance", "AutoExposure", "true"),
        ("Output", "DisplayResolution", "auto"),
        ("Output", "RenderingResolution", "auto"),
        ("Advanced", "MotionVectors", "true"),
        ("Advanced", "ExposureScale", "1.0"),
        ("Advanced", "AutoBias", "true"),
        (
            "GPU",
            "GPUType",
            match vendor {
                GpuVendor::Nvidia => "nvidia",
                GpuVendor::Amd => "amd",
                GpuVendor::Intel => "intel",
                GpuVendor::Unknown => "auto",
            },
        ),
    ]);
    settings
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Unknown,
}

impl GpuVendor {
    /// Map a PCI vendor id (e.g. from wgpu AdapterInfo) to a vendor.
    pub fn from_pci_vendor_id(id: u32) -> Self {
        match id {
            0x10DE => GpuVendor::Nvidia,
            0x1002 => GpuVendor::Amd,
            0x8086 => GpuVendor::Intel,
            _ => GpuVendor::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            GpuVendor::Nvidia => "NVIDIA",
            GpuVendor::Amd => "AMD",
            GpuVendor::Intel => "Intel",
            GpuVendor::Unknown => "Unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[Upscalers]
; Select upscaler for Dx12 games
; 0 = FSR2 | 1 = FSR3 | 2 = XeSS
Dx12Upscaler=1

[Sharpness]
; true or false - enable sharpening
OverrideSharpness=auto
; Sharpness value between 0.0 and 1.0
Sharpness=0.5

[Log]
LogLevel=2
LogFile=auto ; inline comment here
"#;

    #[test]
    fn parses_sections_and_kinds() {
        let doc = parse(SAMPLE);
        assert_eq!(doc.sections.len(), 3);

        let upscaler = doc.get("Upscalers", "Dx12Upscaler").unwrap();
        match &upscaler.kind {
            ValueKind::Options(opts) => {
                assert_eq!(opts.get("0").map(String::as_str), Some("FSR2"));
                assert_eq!(opts.get("2").map(String::as_str), Some("XeSS"));
            }
            other => panic!("expected options, got {other:?}"),
        }

        // Python parity: value "auto" + "auto" anywhere in the comment wins
        // as BoolOptions even when an options run is present
        assert_eq!(
            infer_kind("auto", "0 = FSR2 | 1 = FSR3\nDefault (auto) is FSR3"),
            ValueKind::BoolOptions
        );
        // …but a numeric value with "default (auto)" gets auto injected into
        // the enum options
        match infer_kind("1", "0 = FSR2 | 1 = FSR3\nDefault (auto) is FSR3") {
            ValueKind::Options(opts) => {
                assert_eq!(opts.get("auto").map(String::as_str), Some("auto"));
            }
            other => panic!("expected options with auto, got {other:?}"),
        }

        let sharp_toggle = doc.get("Sharpness", "OverrideSharpness").unwrap();
        assert_eq!(sharp_toggle.kind, ValueKind::BoolOptions);

        let sharpness = doc.get("Sharpness", "Sharpness").unwrap();
        assert_eq!(sharpness.kind, ValueKind::Float);

        let level = doc.get("Log", "LogLevel").unwrap();
        assert_eq!(level.kind, ValueKind::Int);

        // inline comment stripped
        assert_eq!(doc.get("Log", "LogFile").unwrap().value, "auto");
    }

    #[test]
    fn roundtrip_preserves_comments_and_values() {
        let doc = parse(SAMPLE);
        let text = serialize(&doc);
        assert!(text.contains("; Select upscaler for Dx12 games"));
        assert!(text.contains("; 0 = FSR2 | 1 = FSR3 | 2 = XeSS"));
        assert!(text.contains("Dx12Upscaler=1"));
        // reparse gives identical structure
        let doc2 = parse(&text);
        assert_eq!(doc2.sections.len(), doc.sections.len());
        assert_eq!(
            doc2.get("Sharpness", "Sharpness").unwrap().value,
            doc.get("Sharpness", "Sharpness").unwrap().value
        );
    }

    #[test]
    fn write_creates_backup() {
        let tmp = tempfile::tempdir().unwrap();
        let ini = tmp.path().join("OptiScaler.ini");
        std::fs::write(&ini, SAMPLE).unwrap();
        let mut doc = read_file(&ini).unwrap();
        assert!(doc.set_value("Sharpness", "Sharpness", "0.8"));
        write_file(&ini, &doc).unwrap();
        assert!(tmp.path().join("OptiScaler.ini.backup").exists());
        let back = read_file(&ini).unwrap();
        assert_eq!(back.get("Sharpness", "Sharpness").unwrap().value, "0.8");
        // backup holds the OLD value
        let backup = std::fs::read_to_string(tmp.path().join("OptiScaler.ini.backup")).unwrap();
        assert!(backup.contains("Sharpness=0.5"));
    }

    #[test]
    fn auto_settings_apply_only_where_keys_exist() {
        let mut doc = parse("[Fsr]\nEnabled=auto\n\n[GPU]\nGPUType=auto\n");
        let mut applied = 0;
        for (section, key, value) in auto_settings(GpuVendor::Amd) {
            if doc.set_value(section, key, value) {
                applied += 1;
            }
        }
        assert_eq!(applied, 2); // Fsr.Enabled + GPU.GPUType — nothing invented
        assert_eq!(doc.get("Fsr", "Enabled").unwrap().value, "true");
        assert_eq!(doc.get("GPU", "GPUType").unwrap().value, "amd");
    }

    #[test]
    fn vendor_from_pci_id() {
        assert_eq!(GpuVendor::from_pci_vendor_id(0x10DE), GpuVendor::Nvidia);
        assert_eq!(GpuVendor::from_pci_vendor_id(0x1002), GpuVendor::Amd);
        assert_eq!(GpuVendor::from_pci_vendor_id(0x8086), GpuVendor::Intel);
        assert_eq!(GpuVendor::from_pci_vendor_id(0x1234), GpuVendor::Unknown);
    }
}
