use std::io;

use crate::{
    Result,
    postgres::{
        BackendProtocol, backend,
        frontend::{self, FrontendProtocol},
    },
    statement::StatementName,
    transport::{PgTransport, PgTransportExt},
};

pub struct Transaction<IO>
where
    IO: PgTransport
{
    io: IO,
    commited: bool,
}

impl<IO> Transaction<IO>
where
    IO: PgTransport
{
    pub async fn begin(mut io: IO) -> Result<Self> {
        io.send(frontend::Query { sql: "BEGIN" });
        io.flush().await?;
        io.recv::<backend::CommandComplete>().await?;
        let r = io.recv::<backend::ReadyForQuery>().await?;
        assert_eq!(r.tx_status,b'T');
        Ok(Self { io, commited: false })
    }

    pub async fn commit(mut self) -> Result<()> {
        self.io.send(frontend::Query { sql: "COMMIT" });
        self.io.flush().await?;
        self.io.recv::<backend::CommandComplete>().await?;
        let r = self.io.recv::<backend::ReadyForQuery>().await?;
        assert_eq!(r.tx_status,b'I');
        self.commited = true;
        Ok(())
    }
}

impl<IO> Drop for Transaction<IO>
where
    IO: PgTransport
{
    fn drop(&mut self) {
        if !self.commited {
            self.io.send(frontend::Query { sql: "ROLLBACK" });
            self.io.ready_request();
        }
    }
}

impl<IO> PgTransport for Transaction<IO>
where
    IO: PgTransport
{
    fn poll_flush(&mut self, cx: &mut std::task::Context) -> std::task::Poll<io::Result<()>> {
        IO::poll_flush(&mut self.io, cx)
    }

    fn poll_recv<B: BackendProtocol>(&mut self, cx: &mut std::task::Context) -> std::task::Poll<Result<B>> {
        IO::poll_recv(&mut self.io, cx)
    }

    fn ready_request(&mut self) {
        IO::ready_request(&mut self.io)
    }

    fn send<F: FrontendProtocol>(&mut self, message: F) {
        IO::send(&mut self.io, message)
    }

    fn send_startup(&mut self, startup: frontend::Startup) {
        IO::send_startup(&mut self.io, startup)
    }

    fn get_stmt(&mut self, sql: u64) -> Option<StatementName> {
        IO::get_stmt(&mut self.io, sql)
    }

    fn add_stmt(&mut self, sql: u64, id: StatementName) {
        IO::add_stmt(&mut self.io, sql, id)
    }
}

