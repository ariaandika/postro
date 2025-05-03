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
use crate::{Connection, Result, common::trace};

const HALF_MINUTE: Duration = Duration::from_secs(3);

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

    pub fn poll_acquire(&mut self, cx: &mut Context) -> Poll<Result<Connection>> {
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

    pub fn release(&self, conn: Connection) {
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
    conn: Connection,
}

impl PoolConnection {
    fn new(conn: Connection, instant: Instant) -> Self {
        Self {
            healthc_at: instant,
            conn
        }
    }

    fn should_healthcheck(&self) -> bool {
        self.healthc_at.elapsed() > HALF_MINUTE
    }

    fn poll_healthcheck(&mut self, cx: &mut Context) -> Poll<Result<()>> {
        self.conn.poll_ready(cx)
    }

    fn poll_shutdown(&mut self, cx: &mut Context) -> Poll<io::Result<()>> {
        self.conn.poll_shutdown(cx)
    }
}

type AcquireSend = oneshot::Sender<Result<Connection>>;
type AcquireRecv = oneshot::Receiver<Result<Connection>>;

enum WorkerMessage {
    Acquire(AcquireSend),
    Release(Connection),
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
    ///
    /// front queue is the most fresh connection
    conns: VecDeque<PoolConnection>,
    queue: VecDeque<AcquireSend>,

    sleep: Pin<Box<Sleep>>,
    recv: UnboundedReceiver<WorkerMessage>,

    connecting: Option<ConnectFuture>,
    healthcheck: Option<PoolConnection>,
    closing: Option<PoolConnection>,
}

type ConnectFuture = Pin<Box<dyn Future<Output = Result<Connection>> + Send + Sync + 'static>>;

/// Reset `sleep` to the least time to get to the next healthcheck
fn reset_sleep_time(conns: &VecDeque<PoolConnection>, sleep: Pin<&mut Sleep>) {
    let least_time_hc = conns.iter().fold(HALF_MINUTE, |acc, n| {
        (HALF_MINUTE.saturating_sub(n.healthc_at.elapsed())).min(acc)
    });

    trace!("Cycle reset to: {least_time_hc:?}");

    sleep.reset(Instant::now() + least_time_hc);
}

/// Handle connection that is not yet in idle queue.
fn new_connection(
    mut conn: Connection,
    queue: &mut VecDeque<AcquireSend>,
    conns: &mut VecDeque<PoolConnection>,
    instant: Instant,
    is_fresh: bool,
) {
    while let Some(send) = queue.pop_front() {
        if let Err(Ok(_conn)) = send.send(Ok(conn)) {
            conn = _conn;
            continue;
        }

        return;
    }

    if is_fresh {
        conns.push_front(PoolConnection::new(conn, instant));
    } else {
        conns.push_back(PoolConnection::new(conn, instant));
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
            let Some(msg) = msg else {
                // all Pools handle are dropped
                return Poll::Ready(())
            };

            use WorkerMessage::*;
            match msg {
                Acquire(send) => {
                    match conns.pop_front() {
                        Some(conn) => {
                            let hc = conn.healthc_at;
                            if let Err(Ok(conn)) = send.send(Ok(conn.conn)) {
                                conns.push_front(PoolConnection::new(conn, hc));
                            }
                        },
                        None => {
                            queue.push_back(send);
                            if connecting.is_none() && *actives < config.max_conn {
                                *connecting = Some(Box::pin(Connection::connect_with(config.conn.clone())));
                            }
                        },
                    }

                    tracew!("Acquired");
                },
                Release(mut conn) => {
                    if healthcheck.is_none() {
                        // `poll_ready` is most likely to resolved in one poll
                        match conn.poll_ready(cx) {
                            Poll::Ready(Ok(_)) => {
                                new_connection(conn, queue, conns, Instant::now(), true);
                            },
                            Poll::Ready(Err(_err)) => {
                                #[cfg(feature = "log")]
                                log::error!("healthcheck error: {_err}");

                                if closing.is_some() {
                                    drop(conn);
                                } else {
                                    *closing = Some(PoolConnection::new(conn, *started));
                                }
                            },
                            Poll::Pending => {
                                *healthcheck = Some(PoolConnection::new(conn, *started));
                            },
                        }
                    } else {
                        new_connection(conn, queue, conns, *started, false);
                    }

                    tracew!("Released");
                }
            }
        }

        if let Some(Poll::Ready(result)) = connecting.as_mut().map(|e|e.as_mut().poll(cx)) {
            connecting.take();
            match result {
                Ok(conn) => {
                    *actives += 1;
                    new_connection(conn, queue, conns, Instant::now(), true);

                    tracew!("New");
                },
                Err(err) => {
                    #[cfg(feature = "log")]
                    log::error!("failed to connect: {err}");

                    if let Some(send) = queue.pop_front() {
                        let _ = send.send(Err(err));
                    }

                    // TODO: fail connect backpressure instead of immediate error
                },
            }
        }

        if let Some(Poll::Ready(result)) = healthcheck.as_mut().map(|e|e.poll_healthcheck(cx)) {
            let conn = healthcheck.take().unwrap();
            match result {
                Ok(()) => {
                    new_connection(conn.conn, queue, conns, Instant::now(), true);
                },
                Err(_err) => {
                    #[cfg(feature = "log")]
                    log::error!("healthcheck error: {_err}");

                    if closing.is_some() {
                        drop(conn);
                    } else {
                        *closing = Some(conn);
                    }
                },
            }

            // there maybe canceled healthcheck on connection release or healthcheck interval
            reset_sleep_time(conns, sleep.as_mut());

            tracew!("Healthchecked");
        }

        if let Some(Poll::Ready(result)) = closing.as_mut().map(|e|e.poll_shutdown(cx)) {
            let _conn = closing.take().unwrap();

            if let Err(_err) = result {
                #[cfg(feature = "log")]
                log::error!("close error: {_err}");
            }

            *actives -= 1;

            tracew!("Closed");
        }

        if let Poll::Ready(()) = sleep.as_mut().poll(cx) {
            // healthcheck success will call this back
            if healthcheck.is_none() {

                if let Some(i) = conns.iter().rev().position(|e|e.should_healthcheck()) {
                    let mut conn = conns.swap_remove_back(i).unwrap();

                    reset_sleep_time(conns, sleep.as_mut());

                    // Healthcheck can possibly `Ready` in one poll
                    match dbg!(conn.poll_healthcheck(cx)) {
                        Poll::Ready(Ok(_)) => {
                            new_connection(conn.conn, queue, conns, Instant::now(), true);
                        },
                        Poll::Ready(Err(_err)) => {
                            #[cfg(feature = "log")]
                            log::error!("healthcheck error: {_err}");

                            if closing.is_some() {
                                drop(conn);
                            } else {
                                *closing = Some(conn);
                            }
                        },
                        Poll::Pending => {
                            *healthcheck = Some(conn);
                        },
                    }

                } else {
                    reset_sleep_time(conns, sleep.as_mut());
                }
            }

            tracew!("Cycled");
        }

        while let Some(send) = queue.pop_front() {
            match conns.pop_front() {
                Some(conn) => {
                    let hc = conn.healthc_at;
                    if let Err(Ok(conn)) = send.send(Ok(conn.conn)) {
                        conns.push_front(PoolConnection::new(conn, hc));
                    }
                },
                None => {
                    queue.push_front(send);
                    if connecting.is_none() && *actives < config.max_conn {
                        *connecting = Some(Box::pin(Connection::connect_with(config.conn.clone())));
                    }
                    break;
                },
            }
        }

        trace!("{:-<11}: Backpressured: {}", "", queue.len());

        Poll::Pending
    }
}

