use std::{
    collections::VecDeque,
    io,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tokio::{
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender}, oneshot, Notify
    },
    time::{sleep, Instant, Sleep},
};

use super::PoolConfig;
use crate::{PgConnection, Result};

const HALF_MINUTE: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct WorkerHandle {
    send: UnboundedSender<WorkerMessage>,
}

impl WorkerHandle {
    pub fn new(config: PoolConfig) -> (Self, WorkerFuture) {
        let (send, recv) = mpsc::unbounded_channel();
        (
            Self { send },
            WorkerFuture {
                started: Instant::now(),
                config,
                actives: 0,
                conns: VecDeque::new(),
                sleep: Box::pin(sleep(HALF_MINUTE)),
                notify: Arc::new(Notify::new()),
                recv,
                connecting: None,
                healthcheck: None,
                closing: None,
            },
        )
    }

    pub async fn acquire(&self) -> Result<PgConnection> {
        loop {
            let (tx,rx) = oneshot::channel();
            self.send.send(WorkerMessage::Acquire(tx)).expect("worker task closed");
            let notif = match rx.await.expect("worker task closed") {
                Ok(ok) => return Ok(ok),
                Err(err) => err,
            };
            notif.notified().await;
        }
    }

    pub fn release(&self, conn: PgConnection) {
        self.send.send(WorkerMessage::Release(conn)).expect("worker task closed");
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

    fn unhealthy(conn: PgConnection, instant: Instant) -> Self {
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

type AcquireSend = oneshot::Sender<Result<PgConnection,Arc<Notify>>>;

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
    notify: Arc<Notify>,
    recv: UnboundedReceiver<WorkerMessage>,

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

impl Future for WorkerFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let WorkerFuture {
            started, config, actives, sleep, conns, notify, recv,
            connecting, healthcheck, closing
        } = self.as_mut().get_mut();

        while let Poll::Ready(msg) = recv.poll_recv(cx) {
            use WorkerMessage::*;
            let Some(msg) = msg else {
                return Poll::Ready(()) // all Pools are dropped
            };
            match msg {
                Acquire(send) => {
                    // look for available idle conn
                    if let Some(conn) = conns.pop_front() {
                        // conn available, send it to client
                        if let Err(Ok(conn)) = send.send(Ok(conn.conn)) {
                            // client closed, put conn back to pool
                            conns.push_front(PoolConnection::new(conn));
                        }
                    } else {
                        // no idle connection,
                        // tell client to wait for notification
                        // when new conn or released conn
                        if send.send(Err(notify.clone())).is_err() {
                            // client closed
                            continue;
                        }

                        if connecting.is_none() && *actives < config.max_conn {
                            // can create new conn
                            connecting.replace(Box::pin(PgConnection::connect_with(config.conn.clone())));
                        }
                    }
                },
                Release(conn) => {
                    if healthcheck.is_none() {
                        healthcheck.replace(PoolConnection::new(conn));
                    } else {
                        conns.push_back(PoolConnection::unhealthy(conn, *started));
                    }
                }
            }
        }

        if let Some(Poll::Ready(result)) = connecting.as_mut().map(|e|e.as_mut().poll(cx)) {
            connecting.take();
            match result {
                Ok(conn) => {
                    // store connection
                    conns.push_back(PoolConnection::new(conn));
                    *actives += 1;

                    // maybe there is one wating for new conn
                    notify.notify_one();
                },
                Err(err) => {
                    eprintln!("failed to connect: {err}");
                    todo!("backpressure to try reconnect later")
                },
            }
        }

        if let Some(Poll::Ready(result)) = healthcheck.as_mut().map(|e|e.poll_healthcheck(cx)) {
            let conn = healthcheck.take().unwrap();
            match result {
                Ok(()) => {
                    // health ok, store connection
                    conns.push_back(conn);
                },
                Err(err) => {
                    eprintln!("healthcheck error: {err}");
                    // health not ok, close connection
                    if closing.is_some() {
                        eprintln!("ungracefull shutdown");
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
        }

        Poll::Pending
    }
}

