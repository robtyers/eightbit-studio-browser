//! ROM-free, process-free BASIC build demonstration. This does not run an emulator.

use eightbit_dragon_mobile::AuthoringSession;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut session = AuthoringSession::new()?;
    session.edit(1, "10 PRINT \"HELLO IPAD\"\n20 GOTO 10\n".into())?;
    let summary = session.build(1)?;
    println!("Target: {}", summary.target);
    println!("Source revision: {}", summary.source_revision);
    println!("Token bytes: {}", summary.token_bytes);
    println!("Cassette bytes: {}", summary.cassette_bytes);
    println!("Cassette SHA-256: {}", summary.cassette_sha256);
    session.close();
    Ok(())
}
