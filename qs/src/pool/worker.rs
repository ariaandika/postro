use std::{
    collections::VecDeque,
    io,
    pin::Pin,
    task::{Context, Poll, ready},
    time::Duration,
};
use tokio::{
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    time::{Instant, Sleep, sleep},
};

use super::PoolConfig;
use crate::{PgConnection, Result, common::trace};

const HALF_MINUTE: Duration = Duration::from_secs(30);

pub struct WorkerHandle {
    send: UnboundedSender<WorkerMessage>,
    state: State,
}

enum State {
    Idle,
    Recv(AcquireRecv),
}

impl WorkerHandle {
    pub fn new(config: PoolConfig) -> (Self, WorkerFuture) {
        let (send, recv) = mpsc::unbounded_channel();
        (
            Self { send, state: State::Idle },
            WorkerFuture {
                started: Instant::now(),
                config,
                actives: 0,
                conns: VecDeque::new(),
                sleep: Box::pin(sleep(HALF_MINUTE)),
                recv,
                queue: VecDeque::with_capacity(1),
                connecting: None,
                healthcheck: None,
                closing: None,
            },
        )
    }

    pub fn poll_acquire(&mut self, cx: &mut Context) -> Poll<Result<PgConnection>> {
        loop {
            match &mut self.state {
                State::Idle => {
                    let (tx,rx) = oneshot::channel();
                    self.send.send(WorkerMessage::Acquire(tx)).expect("worker task closed");
                    self.state = State::Recv(rx);
                }
                State::Recv(recv) => {
                    let pin = Pin::new(recv);
                    let result = ready!(oneshot::Receiver::poll(pin, cx)).expect("worker pool closed");
                    self.state = State::Idle;
                    return Poll::Ready(result);
                }
            }
        }
    }

    pub fn release(&self, conn: PgConnection) {
        self.send.send(WorkerMessage::Release(conn)).expect("worker task closed");
    }
}

impl Clone for WorkerHandle {
    fn clone(&self) -> Self {
        Self {
            send: self.send.clone(),
            state: State::Idle,
        }
    }
}

impl std::fmt::Debug for WorkerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WorkerHandle")
    }
}

struct PoolConnection {
    healthc_at: Instant,
    conn: PgConnection,
}

impl PoolConnection {
    fn new(conn: PgConnection) -> Self {
        Self {
            healthc_at: Instant::now(),
            conn
        }
    }

    fn with_last_hc(conn: PgConnection, instant: Instant) -> Self {
        Self {
            healthc_at: instant,
            conn
        }
    }

    fn should_healthcheck(&self) -> bool {
        self.healthc_at.elapsed() > HALF_MINUTE
    }

    fn poll_healthcheck(&mut self, cx: &mut Context) -> Poll<Result<()>> {
        self.conn.poll_healthcheck(cx)
    }

    fn poll_shutdown(&mut self, cx: &mut Context) -> Poll<io::Result<()>> {
        self.conn.poll_shutdown(cx)
    }
}

type AcquireSend = oneshot::Sender<Result<PgConnection>>;
type AcquireRecv = oneshot::Receiver<Result<PgConnection>>;

enum WorkerMessage {
    Acquire(AcquireSend),
    Release(PgConnection),
}

pub struct WorkerFuture {
    config: PoolConfig,
    started: Instant,

    actives: usize,
    /// - new conn is pushed back
    /// - acquire conn is poped front
    /// - released conn is pushed back
    /// - healthcheck is swap taken out from the front with the back
    /// - healthcheck ok is pushed front
    conns: VecDeque<PoolConnection>,

    sleep: Pin<Box<Sleep>>,
    recv: UnboundedReceiver<WorkerMessage>,
    queue: VecDeque<AcquireSend>,

    connecting: Option<ConnectFuture>,
    healthcheck: Option<PoolConnection>,
    closing: Option<PoolConnection>,
}

type ConnectFuture = Pin<Box<dyn Future<Output = Result<PgConnection>> + Send + Sync + 'static>>;

/// Reset `sleep` to the least time to get to the next healthcheck
fn reset_sleep_time(conns: &VecDeque<PoolConnection>, sleep: Pin<&mut Sleep>) {
    let least_time_hc = conns.iter().fold(HALF_MINUTE, |acc, n| {
        (HALF_MINUTE.saturating_sub(n.conn.created_at().elapsed())).min(acc)
    });

    sleep.reset(Instant::now() + least_time_hc);
}

fn connection_idle(
    conn: PgConnection,
    queue: &mut VecDeque<AcquireSend>,
    conns: &mut VecDeque<PoolConnection>,
    hc: Instant,
) {
    match queue.pop_front() {
        Some(send) => {
            if let Err(Ok(conn)) = send.send(Ok(conn)) {
                conns.push_back(PoolConnection::with_last_hc(conn, hc));
            }
        }
        None => {
            conns.push_back(PoolConnection::with_last_hc(conn, hc));
        }
    }
}

impl Future for WorkerFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let WorkerFuture {
            started, config, actives, sleep, conns,
            recv, queue,
            connecting, healthcheck, closing
        } = self.as_mut().get_mut();

        macro_rules! tracew {
            ($prefix:literal) => {
                trace!(
                    "{:11}: Active={actives}, Idle={}, Connecting={}, Healthcheck={}, Closing={}",
                    $prefix,
                    conns.len(),
                    connecting.is_some() as u8,
                    healthcheck.is_some() as u8,
                    closing.is_some() as u8,
                );
            };
        }

        // PERF: maybe we can have multiple slot for connecting futures ?

        // NOTE:
        // 1. Collect all request upfront
        // 2. Poll any connection futures
        // With the highest chance of connection available:
        // 3. Try to fulfill Queues

        while let Poll::Ready(msg) = recv.poll_recv(cx) {
            use WorkerMessage::*;
            let Some(msg) = msg else {
                return Poll::Ready(()) // all Pools are dropped
            };
            match msg {
                Acquire(send) => {
                    tracew!("Acquire");

                    match conns.pop_front() {
                        // look for available idle conn
                        Some(conn) => {
                            // conn available, send it to client
                            if let Err(Ok(conn)) = send.send(Ok(conn.conn)) {
                                // client closed, put conn back to pool
                                conns.push_front(PoolConnection::new(conn));
                            }
                        },
                        None => {
                            // no idle connection,
                            // push into queue
                            queue.push_back(send);

                            if connecting.is_none() && *actives < config.max_conn {
                                // no connecting in progress, and under max connection
                                connecting.replace(Box::pin(PgConnection::connect_with(config.conn.clone())));
                            }
                        },
                    }
                },
                Release(conn) => {
                    tracew!("Released");

                    // released conn always immediately healthchecked
                    if healthcheck.is_none() {
                        healthcheck.replace(PoolConnection::new(conn));
                    } else {
                        // other healthcheck is in progress
                        connection_idle(conn, queue, conns, *started);
                    }
                }
            }
        }

        if let Some(Poll::Ready(result)) = connecting.as_mut().map(|e|e.as_mut().poll(cx)) {
            connecting.take();
            match result {
                Ok(conn) => {
                    *actives += 1;
                    connection_idle(conn, queue, conns, Instant::now());
                    tracew!("New");
                },
                Err(err) => {
                    #[cfg(feature = "log")]
                    log::error!("failed to connect: {err}");

                    todo!("backpressure to try reconnect later")
                },
            }
        }

        if let Some(Poll::Ready(result)) = healthcheck.as_mut().map(|e|e.poll_healthcheck(cx)) {
            let conn = healthcheck.take().unwrap();
            match result {
                Ok(()) => {
                    // health ok, store connection
                    connection_idle(conn.conn, queue, conns, Instant::now());
                    tracew!("Healthcheck");
                },
                Err(err) => {
                    #[cfg(feature = "log")]
                    log::error!("healthcheck error: {err}");

                    // health not ok, close connection
                    if closing.is_some() {
                        drop(conn);
                    } else {
                        closing.replace(conn);
                    }
                },
            }
            // there maybe delayed healthcheck
            reset_sleep_time(conns, sleep.as_mut());
        }

        if let Some(Poll::Ready(result)) = closing.as_mut().map(|e|e.poll_shutdown(cx)) {
            let _conn = closing.take().unwrap();
            if let Err(err) = result {
                eprintln!("close error: {err}");
            }
            *actives -= 1;

            tracew!("Closed");
        }

        if let Poll::Ready(()) = sleep.as_mut().poll(cx) {
            if let Some(i) = conns.iter().rev().position(|e|e.should_healthcheck()) {
                let conn = conns.swap_remove_back(i).expect("iterated");

                reset_sleep_time(conns, sleep.as_mut());

                if healthcheck.is_none() {
                    healthcheck.replace(conn);
                } else {
                    conns.push_back(conn);
                }

            } else {
                reset_sleep_time(conns, sleep.as_mut());
            }

            tracew!("Cycle");
        }

        while let Some(send) = queue.pop_front() {
            match conns.pop_front() {
                Some(conn) => {
                    if let Err(Ok(conn)) = send.send(Ok(conn.conn)) {
                        conns.push_front(PoolConnection::new(conn));
                    }
                },
                None => {
                    queue.push_front(send);
                    if connecting.is_none() && *actives < config.max_conn {
                        connecting.replace(Box::pin(PgConnection::connect_with(config.conn.clone())));
                    }
                    break
                },
            }
        }

        trace!("{:-<11}: Backpressured: {}", "", queue.len());

        Poll::Pending
    }
}

