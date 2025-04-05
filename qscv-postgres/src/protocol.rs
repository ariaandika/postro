use crate::{io::PostgresIo, message::frontend::Startup};


/// perform a startup message
///
/// <https://www.postgresql.org/docs/17/protocol-flow.html#PROTOCOL-FLOW-START-UP>
pub fn startup<IO: PostgresIo>(sql: &str, io: IO) {
    // io.send_startup(Startup {
    //     user: &opt.user,
    //     database: Some(&opt.dbname),
    //     replication: None,
    // });
}

