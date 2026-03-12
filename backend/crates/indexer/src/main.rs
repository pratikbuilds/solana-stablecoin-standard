use anyhow::Result;

fn main() -> Result<()> {
    let report = sss_db::health_report("indexer", "backend");
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
