use std::fs::File;

use anyhow::Result;

pub fn show_status() -> Result<File> {
    // Open Socket

    // Send `show status\n`

    let file = File::open("tests/birdc/show-status")?;
    Ok(file)
}

pub fn show_protocols_all() -> Result<File> {
    let file = File::open("tests/birdc/show-protocols-all")?;
    Ok(file)
}

pub fn show_route_all_protocol(id: &str) -> Result<File> {
    println!("show route all protocol '{}'", id);
    let file = File::open("tests/birdc/show-route-all-protocol-R192_175")?;
    Ok(file)
}

pub fn show_route_all_protocol_filtered(id: &str) -> Result<File> {
    println!("show route all filtered protocol '{}'", id);
    let file = File::open("tests/birdc/show-route-all-filtered-protocol-R193_51")?;
    Ok(file)
}

pub fn show_route_all_protocol_noexport(id: &str) -> Result<File> {
    println!("show route all noexport protocol '{}'", id);
    let file = File::open("tests/birdc/show-route-all-noexport-protocol-R193_51")?;
    Ok(file)
}

pub fn show_route_all_table(table: &str) -> Result<File> {
    println!("show route all table '{}'", table);
    let file = File::open("tests/birdc/show-route-all-table-master4")?;
    Ok(file)
}

pub fn show_route_all_table_filtered(table: &str) -> Result<File> {
    println!("show route all filtered table '{}'", table);
    let file = File::open("tests/birdc/show-route-all-filtered-table-master4")?;
    Ok(file)
}
