use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;

// The ledger entries are frozen in Git. Supply the three WASMs separately so
// the reviewer can get them directly from Stellar by their recorded hashes.
pub(super) fn restore(template: &str) -> NamedTempFile {
    let binaries: [&[u8]; 3] = [
        super::sushi::WASM,
        include_bytes!("../../wasm/9f94c577.wasm"),
        include_bytes!("../../wasm/aba81900.wasm"),
    ];
    let mut snapshot: Value = serde_json::from_str(template).expect("valid fixture JSON");
    let entries = snapshot["ledger_snapshot"]["ledger_entries"]
        .as_array_mut()
        .expect("ledger entries");
    for code in entries
        .iter_mut()
        .filter_map(|entry| entry.pointer_mut("/entry/data/contract_code"))
    {
        assert_eq!(code["code"], "", "fixture must omit contract bytes");
        let hash = code["hash"].as_str().expect("contract code hash");
        let wasm = binaries
            .iter()
            .find(|wasm| hex::encode(Sha256::digest(wasm)) == hash)
            .unwrap_or_else(|| panic!("missing or incorrect WASM for {hash}"));
        code["code"] = Value::String(hex::encode(wasm));
    }
    let file = NamedTempFile::new().expect("temporary fixture file");
    serde_json::to_writer(file.as_file(), &snapshot).expect("write local fixture");
    file
}
