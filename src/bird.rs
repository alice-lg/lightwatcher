use std::io::Write;
use std::os::unix::net::UnixStream;

use anyhow::Result;

use crate::config;

pub enum Command {
    ShowStatus,
    ShowProtocolsAll,
    ShowRouteAllProtocol(String),
    ShowRouteAllFilteredProtocol(String),
    ShowRouteAllNoexportProtocol(String),
    ShowRouteAllTable(String),
    ShowRouteAllFilteredTable(String),
}

/// Remove potentially harmful characters from the string
fn sanitize_userdata(s: String) -> String {
    s.replace("'", "_")
        .replace("`", "_")
        .replace("\"", "_")
        .replace("\n", "_")
        .replace("\t", "_")
        .replace(",", "_")
        .replace(";", "_")
}

impl Into<String> for Command {
    fn into(self) -> String {
        match self {
            Command::ShowStatus => "show status\n".to_string(),
            Command::ShowProtocolsAll => "show protocols all\n".to_string(),
            Command::ShowRouteAllProtocol(id) => {
                let id = sanitize_userdata(id);
                format!("show route all protocol '{}'\n", id)
            }
            Command::ShowRouteAllFilteredProtocol(id) => {
                let id = sanitize_userdata(id);
                format!("show route all filtered protocol '{}'\n", id)
            }
            Command::ShowRouteAllNoexportProtocol(id) => {
                let id = sanitize_userdata(id);
                format!("show route all noexport protocol '{}'\n", id)
            }
            Command::ShowRouteAllTable(table) => {
                let table = sanitize_userdata(table);
                format!("show route all table '{}'\n", table)
            }
            Command::ShowRouteAllFilteredTable(table) => {
                let table = sanitize_userdata(table);
                format!("show route all filtered table '{}'\n", table)
            }
        }
    }
}

/// Connect to birdc on the unix socket
/// and send the command.
pub fn birdc(cmd: Command) -> Result<UnixStream> {
    let socket_addr = config::get_birdc_socket();
    let mut stream = UnixStream::connect(socket_addr)?;
    let req: String = cmd.into();

    stream.write_all(&req.as_bytes())?;
    Ok(stream)
}

