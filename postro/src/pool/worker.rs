use std::{
    collections::VecDeque,
    pin::Pin,
    task::{
        Context,
        Poll::{self, *},
        ready,
    },
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
use crate::{
    Connection, Result,
    common::{span, verbose},
};

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
    pub fn new(config: PoolConfig) -> (Self, WorkerFutureV2) {
        let (send, recv) = mpsc::unbounded_channel();
        (
            Self { send, state: State::Idle },
            WorkerFutureV2 {
                started: Instant::now(),
                #[cfg(feature = "verbose")]
                iter_n: 0,
                connect_retry: 0,

                actives: 0,
                conns: VecDeque::new(),
                // queue: VecDeque::with_capacity(1),
                acquires: VecDeque::with_capacity(1),
                recv,

                connect_delay: None,
                connecting: None,
                healthcheck: None,
                closing: None,
                sleep: Box::pin(sleep(config.interval)),

                config,
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
    last_hc: Instant,
    conn: Connection,
}

impl PoolConnection {
    fn new(conn: Connection, instant: Instant) -> Self {
        Self {
            last_hc: instant,
            conn
        }
    }

    fn now(conn: Connection) -> Self {
        Self {
            last_hc: Instant::now(),
            conn
        }
    }

    fn should_healthcheck(&self) -> bool {
        self.last_hc.elapsed() > HALF_MINUTE
    }

    fn poll_healthcheck(&mut self, cx: &mut Context) -> Poll<Result<()>> {
        let result = ready!(self.conn.poll_ready(cx));
        if result.is_ok() {
            self.last_hc = Instant::now();
        }
        Poll::Ready(result)
    }
}

type AcquireSend = oneshot::Sender<Result<Connection>>;
type AcquireRecv = oneshot::Receiver<Result<Connection>>;

enum WorkerMessage {
    Acquire(AcquireSend),
    Release(Connection),
}

type ConnectFuture = Pin<Box<dyn Future<Output = Result<Connection>> + Send + Sync + 'static>>;

pub struct WorkerFutureV2 {
    config: PoolConfig,
    started: Instant,
    #[cfg(feature = "verbose")]
    iter_n: u8,

    actives: usize,
    /// - new conn is pushed back
    /// - acquire conn is poped front
    /// - released conn is pushed back
    /// - healthcheck is swap taken out from the front with the back
    /// - healthcheck ok is pushed front
    ///
    /// front queue is the most fresh connection
    conns: VecDeque<PoolConnection>,
    acquires: VecDeque<AcquireSend>,
    recv: UnboundedReceiver<WorkerMessage>,

    connect_retry: usize,
    connect_delay: Option<Pin<Box<Sleep>>>,
    connecting: Option<ConnectFuture>,
    healthcheck: Option<PoolConnection>,
    closing: Option<Connection>,
    sleep: Pin<Box<Sleep>>,
}

impl Future for WorkerFutureV2 {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        #[cfg(feature = "verbose")]
        {
            self.iter_n = self.iter_n.wrapping_add(1);
        }
        span!(
            "worker",
            n=self.iter_n,
        );

        // the only branch that can exit worker
        if self.poll_incoming_message(cx).is_ready() {
            #[cfg(feature = "log")]
            log::info!("worker exit");
            return Ready(());
        }

        // if there is `Release` after `Acquire`
        while !self.acquires.is_empty() {
            span!("acquire-demand");
            match self.poll_connecting(cx) {
                Ready(result) => self.send_acquire_queue(result),
                Pending => break,
            }
        }

        if let Ready(result) = self.poll_connecting(cx) {
            span!("connect-queue");
            self.send_acquire_queue(result);
            while !self.acquires.is_empty() {
                span!("acquire-demand");
                match self.poll_connecting(cx) {
                    Ready(result) => self.send_acquire_queue(result),
                    Pending => break,
                }
            }
        }

        if let Some(conn) = self.healthcheck.take() {
            self.poll_healthcheck(conn, cx);
            while self.healthcheck.is_none() {
                match self.conns.iter().rev().position(PoolConnection::should_healthcheck) {
                    Some(i) => {
                        let conn = self.conns.swap_remove_back(i).unwrap();
                        self.poll_healthcheck(conn, cx);
                    }
                    None => break,
                }
            }
        }

        if let Some(conn) = self.closing.take() {
            self.poll_close(conn, cx);
        }

        if let Poll::Ready(()) = self.sleep.as_mut().poll(cx) {
            verbose!("Interval");
            self.reset_interval();
        }

        verbose!(
            actives=self.actives,
            idle=self.conns.len(),
            hc=self.healthcheck.is_some() as u8,
            interval=?{self.sleep.deadline() - Instant::now()}.as_secs(),
            backpressured=self.acquires.len(),
            "polled"
        );

        Poll::Pending
    }
}

impl WorkerFutureV2 {
    fn poll_incoming_message(&mut self, cx: &mut Context) -> Poll<()> {
        while let Poll::Ready(msg) = self.recv.poll_recv(cx) {
            let Some(msg) = msg else {
                return Poll::Ready(());
            };

            match msg {
                WorkerMessage::Acquire(send) => {
                    span!("acquire");
                    verbose!("Acquire");

                    match self.pop_connection(cx) {
                        Poll::Pending => self.acquires.push_back(send),
                        Poll::Ready(Ok(PoolConnection { last_hc, conn })) => {
                            if let Err(Ok(conn)) = send.send(Ok(conn)) {
                                self.conns.push_back(PoolConnection::new(conn, last_hc));
                            }
                        },
                        Poll::Ready(Err(err)) => send.send(Err(err)).unwrap_or(()),
                    }
                },
                WorkerMessage::Release(conn) => {
                    span!("release");
                    verbose!("Release");

                    self.healthcheck(conn, cx);
                }
            }
        }

        Poll::Pending
    }

    fn pop_connection(&mut self, cx: &mut Context) -> Poll<Result<PoolConnection>>{
        match self.conns.pop_front() {
            Some(ok) => Poll::Ready(Ok(ok)),
            None => self.poll_connecting(cx),
        }
    }

    /// `Ready` returns is always with retry polled
    fn poll_connecting(&mut self, cx: &mut Context) -> Poll<Result<PoolConnection>> {
        match self.connect_delay.as_mut() {
            Some(f) => {
                // wait for `connect_delay: Sleep`
                ready!(f.as_mut().poll(cx));
                self.connect_delay.take();
            },
            None => {},
        }

        if self.connecting.is_none() && self.actives >= self.config.max_conn {
            // wait for `Release`
            verbose!("new connection backpressured");
            return Poll::Pending;
        }

        let poll = self
            .connecting
            .get_or_insert_with(||Box::pin(Connection::connect_with(self.config.conn.clone())))
            .as_mut()
            .poll(cx);

        // wait for `Connection::connect`
        let result = ready!(poll);
        self.connecting.take();

        match result {
            Ok(conn) => {
                self.connect_retry = 0;
                self.actives += 1;
                verbose!(actives=self.actives,"new-connection");
                Poll::Ready(Ok(PoolConnection::now(conn)))
            },
            Err(err) => {
                #[cfg(feature = "log")]
                log::error!("failed to connect: {err:#}, retry={}",self.connect_retry);

                if self.connect_retry < self.config.max_retry {
                    self.connect_retry += 1;
                    self.connect_delay = Some(Box::pin(sleep(self.config.retry_delay)));
                    // wait for `connect_delay: Sleep`
                    Poll::Pending
                } else {
                    self.connect_retry = 0;
                    self.connecting.take();
                    Poll::Ready(Err(err))
                }
            },
        }
    }

    fn healthcheck(&mut self, conn: Connection, cx: &mut Context) {
        if let Some(conn) = self.healthcheck.take() {
            self.poll_healthcheck(conn, cx);
        }
        self.poll_healthcheck(PoolConnection::new(conn, self.started), cx);
    }

    fn poll_healthcheck(&mut self, mut conn: PoolConnection, cx: &mut Context) {
        match conn.poll_healthcheck(cx) {
            Pending if self.healthcheck.is_none() => self.healthcheck = Some(conn),
            Pending => self.conns.push_back(conn),
            Ready(Ok(())) if !self.acquires.is_empty() => self.send_acquire_queue(Ok(conn)),
            Ready(Ok(())) => self.conns.push_front(conn),
            Ready(Err(_err)) => {
                #[cfg(feature = "log")]
                log::error!("connection healthcheck failed: {_err:#}");
                self.close(conn.conn, cx);
            }
        }
    }

    fn send_acquire_queue(&mut self, result: Result<PoolConnection>) {
        match (self.acquires.pop_front(), result) {
            (Some(send), result) => self.send_acquire(send, result),
            (None, Ok(conn)) => self.conns.push_back(conn),
            (None, Err(_)) => {}
        }
    }

    fn send_acquire(&mut self, send: AcquireSend, result: Result<PoolConnection>) {
        match result {
            Ok(PoolConnection { last_hc, conn }) => {
                let Err(Ok(conn)) = send.send(Ok(conn)) else {
                    return;
                };
                if self.acquires.is_empty() {
                    self.conns.push_front(PoolConnection::new(conn, last_hc));
                } else {
                    self.send_acquire_queue(Ok(PoolConnection::new(conn, last_hc)));
                }
            },
            Err(err) => send.send(Err(err)).unwrap_or(()),
        }
    }

    fn reset_interval(&mut self) {
        let least_time_hc = self.conns.iter().fold(self.config.interval, |acc, n| {
            (self.config.interval.saturating_sub(n.last_hc.elapsed())).min(acc)
        });

        self.sleep.as_mut().reset(Instant::now() + least_time_hc);
    }

    fn close(&mut self, conn: Connection, cx: &mut Context) {
        if let Some(conn) = self.closing.take() {
            self.poll_close(conn, cx);
        }
        self.poll_close(conn, cx);
    }

    fn poll_close(&mut self, mut conn: Connection, cx: &mut Context) {
        match conn.poll_shutdown(cx) {
            Ready(_) if {
                self.actives -= 1;
                verbose!("closed");
                false
            } => {}
            Ready(Ok(())) => {}
            Ready(Err(_err)) => {
                #[cfg(feature = "log")]
                log::error!("failed to close connection: {_err:#}");
            }
            Pending if self.closing.is_none() => self.closing = Some(conn),
            Pending => {
                self.actives -= 1;
                verbose!("closed");
            } // connection is not dropped cleanly
        }
    }
}

