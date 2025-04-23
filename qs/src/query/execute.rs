use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use super::{ops, portal::Portal};
use crate::{Result, encode::Encoded, postgres::backend, transport::PgTransport};

pin_project_lite::pin_project! {
    #[derive(Debug)]
    #[project = ExecuteProject]
    pub struct Execute<'sql, 'val, IO> {
        #[pin]
        phase: Phase<'sql, 'val, IO>,
    }
}

pin_project_lite::pin_project! {
    #[derive(Debug)]
    #[project = PhaseProject]
    enum Phase<'sql, 'val, IO> {
        Portal {
            #[pin]
            portal: Portal<'sql, 'val, IO>,
        },
        BindComplete { io: Option<IO> },
        NoData { io: Option<IO> },
        Execute { io: Option<IO> },
        ReadyForQuery { io: IO, row_info: u64 },
        Complete,
    }
}

impl<'sql, 'val, IO> Execute<'sql, 'val, IO> {
    pub(crate) fn new(sql: &'sql str, io: IO, params: Vec<Encoded<'val>>, persistent: bool) -> Self {
        Self {
            phase: Phase::Portal {
                portal: Portal::new(sql, io, params, 0, persistent),
            },
        }
    }
}

impl<IO> Future for Execute<'_, '_, IO>
where
    IO: PgTransport,
{
    type Output = Result<u64>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let ExecuteProject { mut phase, } = self.as_mut().project();
        loop {
            match phase.as_mut().project() {
                PhaseProject::Portal { portal } => {
                    let io = ready!(portal.poll(cx)?);
                    *phase = Phase::BindComplete { io: Some(io) };
                },
                PhaseProject::BindComplete { io } => {
                    ready!(io.as_mut().unwrap().poll_recv::<backend::BindComplete>(cx)?);
                    *phase = Phase::NoData { io: io.take() };
                },
                PhaseProject::NoData { io } => {
                    ready!(io.as_mut().unwrap().poll_recv::<backend::NoData>(cx)?);
                    *phase = Phase::Execute { io: io.take() };
                }
                PhaseProject::Execute { io } => {
                    let cmd = ready!(io.as_mut().unwrap().poll_recv::<backend::CommandComplete>(cx)?);
                    let row_info = ops::command_complete(cmd);
                    *phase = Phase::ReadyForQuery { io: io.take().unwrap(), row_info };
                },
                PhaseProject::ReadyForQuery { io, row_info } => {
                    ready!(io.poll_recv::<backend::ReadyForQuery>(cx)?);
                    return Poll::Ready(Ok(*row_info));
                },
                PhaseProject::Complete => panic!("`poll` after complete"),
            }
        }
    }
}

