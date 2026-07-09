//! Scriptable network peer used by conformance tests. Binds a real
//! socket on `127.0.0.1:0` so backends exercise their kernel path.
//! Actions send mock MoldUDP or SoupBinTCP framed bytes and assert the
//! transport's outbound writes.

use std::{net::SocketAddr, ops::Range, time::Duration};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, UdpSocket},
};

const MOCK_SESSION: [u8; 10] = *b"MOCKPEER01";

pub struct MockPeer {
    pub kind: MockKind,
    pub script: Vec<MockAction>,
    pub drop_rate: f32,
    pub jitter: Range<u64>,
    pub target: Option<SocketAddr>,
}

impl MockPeer {
    pub fn new(kind: MockKind, target: Option<SocketAddr>) -> Self {
        Self {
            kind,
            script: Vec::new(),
            drop_rate: 0.0,
            jitter: 0..0,
            target,
        }
    }

    pub fn with_script(mut self, script: Vec<MockAction>) -> Self {
        self.script = script;
        self
    }

    pub async fn run(self) -> Result<MockRunReport, MockPeerError> {
        match self.kind {
            MockKind::Udp { bind } => {
                run_udp(bind, self.script, self.drop_rate, self.jitter, self.target).await
            }
            MockKind::Tcp { bind } => run_tcp(bind, self.script, self.drop_rate, self.jitter).await,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MockKind {
    Udp { bind: SocketAddr },
    Tcp { bind: SocketAddr },
}

#[derive(Debug, Clone)]
pub enum MockAction {
    SendMoldPacket { seq: u64, payload: Vec<u8> },
    SendMoldHeartbeat,
    SendSoupPacket { ty: u8, payload: Vec<u8> },
    ExpectClientBytes(Vec<u8>),
    Sleep(Duration),
}

#[derive(Debug, Default)]
pub struct MockRunReport {
    pub actions_completed: usize,
    pub bytes_sent: u64,
    pub bytes_dropped_synthetic: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum MockPeerError {
    #[error("bind failed: {0}")]
    Bind(#[source] std::io::Error),

    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("UDP script needs a target address for send actions")]
    MissingUdpTarget,

    #[error("TCP script exhausted before an ExpectClientBytes could satisfy")]
    TcpExpectShort,

    #[error("expected {expected:?}, got {got:?}")]
    ExpectMismatch { expected: Vec<u8>, got: Vec<u8> },
}

fn maybe_jitter(jitter: &Range<u64>) -> Option<Duration> {
    if jitter.end <= jitter.start {
        return None;
    }
    let ms = fastrand::u64(jitter.clone());
    Some(Duration::from_millis(ms))
}

fn should_drop(drop_rate: f32) -> bool {
    drop_rate > 0.0 && fastrand::f32() < drop_rate
}

fn encode_mold_data(seq: u64, payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(22 + payload.len());
    buf.extend_from_slice(&MOCK_SESSION);
    buf.extend_from_slice(&seq.to_be_bytes());
    buf.extend_from_slice(&1u16.to_be_bytes());
    buf.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    buf.extend_from_slice(payload);
    buf
}

fn encode_mold_heartbeat() -> Vec<u8> {
    let mut buf = Vec::with_capacity(20);
    buf.extend_from_slice(&MOCK_SESSION);
    buf.extend_from_slice(&0u64.to_be_bytes());
    buf.extend_from_slice(&0u16.to_be_bytes());
    buf
}

fn encode_soup(ty: u8, payload: &[u8]) -> Vec<u8> {
    let len = 1 + payload.len();
    let mut buf = Vec::with_capacity(2 + len);
    buf.extend_from_slice(&(len as u16).to_be_bytes());
    buf.push(ty);
    buf.extend_from_slice(payload);
    buf
}

async fn run_udp(
    bind: SocketAddr,
    script: Vec<MockAction>,
    drop_rate: f32,
    jitter: Range<u64>,
    target: Option<SocketAddr>,
) -> Result<MockRunReport, MockPeerError> {
    let sock = UdpSocket::bind(bind).await.map_err(MockPeerError::Bind)?;
    let mut report = MockRunReport::default();
    let mut recv_buf = vec![0u8; 64 * 1024];

    for action in script {
        match action {
            MockAction::SendMoldPacket { seq, payload } => {
                let bytes = encode_mold_data(seq, &payload);
                send_udp(&sock, &bytes, &mut report, drop_rate, &jitter, target).await?;
            }
            MockAction::SendMoldHeartbeat => {
                let bytes = encode_mold_heartbeat();
                send_udp(&sock, &bytes, &mut report, drop_rate, &jitter, target).await?;
            }
            MockAction::SendSoupPacket { ty, payload } => {
                let bytes = encode_soup(ty, &payload);
                send_udp(&sock, &bytes, &mut report, drop_rate, &jitter, target).await?;
            }
            MockAction::ExpectClientBytes(expected) => {
                let (n, _peer) = sock.recv_from(&mut recv_buf).await?;
                let got = recv_buf[..n].to_vec();
                if got != expected {
                    return Err(MockPeerError::ExpectMismatch { expected, got });
                }
            }
            MockAction::Sleep(d) => tokio::time::sleep(d).await,
        }
        report.actions_completed += 1;
    }
    Ok(report)
}

async fn send_udp(
    sock: &UdpSocket,
    bytes: &[u8],
    report: &mut MockRunReport,
    drop_rate: f32,
    jitter: &Range<u64>,
    target: Option<SocketAddr>,
) -> Result<(), MockPeerError> {
    if let Some(delay) = maybe_jitter(jitter) {
        tokio::time::sleep(delay).await;
    }
    if should_drop(drop_rate) {
        report.bytes_dropped_synthetic += bytes.len() as u64;
        return Ok(());
    }
    let target = target.ok_or(MockPeerError::MissingUdpTarget)?;
    let n = sock.send_to(bytes, target).await?;
    report.bytes_sent += n as u64;
    Ok(())
}

async fn run_tcp(
    bind: SocketAddr,
    script: Vec<MockAction>,
    drop_rate: f32,
    jitter: Range<u64>,
) -> Result<MockRunReport, MockPeerError> {
    let listener = TcpListener::bind(bind).await.map_err(MockPeerError::Bind)?;
    let (mut stream, _peer) = listener.accept().await?;
    let mut report = MockRunReport::default();

    for action in script {
        match action {
            MockAction::SendMoldPacket { seq, payload } => {
                let bytes = encode_mold_data(seq, &payload);
                send_tcp(&mut stream, &bytes, &mut report, drop_rate, &jitter).await?;
            }
            MockAction::SendMoldHeartbeat => {
                let bytes = encode_mold_heartbeat();
                send_tcp(&mut stream, &bytes, &mut report, drop_rate, &jitter).await?;
            }
            MockAction::SendSoupPacket { ty, payload } => {
                let bytes = encode_soup(ty, &payload);
                send_tcp(&mut stream, &bytes, &mut report, drop_rate, &jitter).await?;
            }
            MockAction::ExpectClientBytes(expected) => {
                let mut got = vec![0u8; expected.len()];
                stream
                    .read_exact(&mut got)
                    .await
                    .map_err(|_| MockPeerError::TcpExpectShort)?;
                if got != expected {
                    return Err(MockPeerError::ExpectMismatch { expected, got });
                }
            }
            MockAction::Sleep(d) => tokio::time::sleep(d).await,
        }
        report.actions_completed += 1;
    }
    Ok(report)
}

async fn send_tcp(
    stream: &mut tokio::net::TcpStream,
    bytes: &[u8],
    report: &mut MockRunReport,
    drop_rate: f32,
    jitter: &Range<u64>,
) -> Result<(), MockPeerError> {
    if let Some(delay) = maybe_jitter(jitter) {
        tokio::time::sleep(delay).await;
    }
    if should_drop(drop_rate) {
        report.bytes_dropped_synthetic += bytes.len() as u64;
        return Ok(());
    }
    stream.write_all(bytes).await?;
    report.bytes_sent += bytes.len() as u64;
    Ok(())
}
