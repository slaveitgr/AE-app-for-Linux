use std::io;

use aeapp_resources::aeapp_windows_rc;

fn main() -> io::Result<()> {
    aeapp_windows_rc("aeappbootstrap", "AeApp Launcher")
}
