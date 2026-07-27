use crate::{
    InitProtocol,
    error::{InitProtocolError, ProtocolError},
    frame::InitFrame,
    types::{
        Pid, STREAM_ID_OFFSET1, STREAM_ID_OFFSET2, Sid, VELOREN_MAGIC_NUMBER,
        VELOREN_NETWORK_VERSION,
    },
};
use async_trait::async_trait;
use tracing::{debug, error, info, trace};

/// Implement this for auto Handshake with [`ReliableSink`].
/// You must make sure that EVERY message send this way actually is received on
/// the receiving site:
///  - exactly once
///  - in the correct order
///  - correctly
///
/// [`ReliableSink`]: crate::ReliableSink
/// [`RecvProtocol`]: crate::RecvProtocol
#[async_trait]
pub trait ReliableDrain {
    type CustomErr: std::fmt::Debug + Send;
    async fn send(&mut self, frame: InitFrame) -> Result<(), ProtocolError<Self::CustomErr>>;
}

/// Implement this for auto Handshake with [`ReliableDrain`]. See
/// [`ReliableDrain`].
///
/// [`ReliableDrain`]: crate::ReliableDrain
#[async_trait]
pub trait ReliableSink {
    type CustomErr: std::fmt::Debug + Send;
    async fn recv(&mut self) -> Result<InitFrame, ProtocolError<Self::CustomErr>>;
}

#[async_trait]
impl<D, S, E> InitProtocol for (D, S)
where
    D: ReliableDrain<CustomErr = E> + Send,
    S: ReliableSink<CustomErr = E> + Send,
    E: std::fmt::Debug + Send,
{
    type CustomErr = E;

    async fn initialize(
        &mut self,
        initializer: bool,
        local_pid: Pid,
        local_secret: u128,
    ) -> Result<(Pid, Sid, u128), InitProtocolError<E>> {
        #[cfg(debug_assertions)]
        const WRONG_NUMBER: &str = "Handshake does not contain the magic number required by \
                                    veloren server.\nWe are not sure if you are a valid veloren \
                                    client.\nClosing the connection";
        #[cfg(debug_assertions)]
        const WRONG_VERSION: &str = "Handshake does contain a correct magic number, but invalid \
                                     version.\nWe don't know how to communicate with \
                                     you.\nClosing the connection";
        const ERR_S: &str = "Got A Raw Message, these are usually Debug Messages indicating that \
                             something went wrong on network layer and connection will be closed";

        let drain = &mut self.0;
        let sink = &mut self.1;

        if initializer {
            drain
                .send(InitFrame::Handshake {
                    magic_number: VELOREN_MAGIC_NUMBER,
                    version: VELOREN_NETWORK_VERSION,
                })
                .await?;
        }

        match sink.recv().await? {
            InitFrame::Handshake {
                magic_number,
                version,
            } => {
                trace!(?magic_number, ?version, "Recv handshake");
                if magic_number != VELOREN_MAGIC_NUMBER {
                    error!(?magic_number, "Connection with invalid magic_number");
                    #[cfg(debug_assertions)]
                    drain
                        .send(InitFrame::Raw(WRONG_NUMBER.as_bytes().to_vec()))
                        .await?;
                    Err(InitProtocolError::WrongMagicNumber(magic_number))
                } else if version[0] != VELOREN_NETWORK_VERSION[0]
                    || version[1] != VELOREN_NETWORK_VERSION[1]
                {
                    error!(?version, "Connection with wrong network version");
                    #[cfg(debug_assertions)]
                    drain
                        .send(InitFrame::Raw(
                            format!(
                                "{} Our Version: {:?}\nYour Version: {:?}\nClosing the connection",
                                WRONG_VERSION, VELOREN_NETWORK_VERSION, version,
                            )
                            .as_bytes()
                            .to_vec(),
                        ))
                        .await?;
                    Err(InitProtocolError::WrongVersion(version))
                } else {
                    trace!("Handshake Frame completed");
                    if initializer {
                        drain
                            .send(InitFrame::Init {
                                pid: local_pid,
                                secret: local_secret,
                            })
                            .await?;
                    } else {
                        drain
                            .send(InitFrame::Handshake {
                                magic_number: VELOREN_MAGIC_NUMBER,
                                version: VELOREN_NETWORK_VERSION,
                            })
                            .await?;
                    }
                    Ok(())
                }
            },
            InitFrame::Raw(bytes) => {
                match std::str::from_utf8(bytes.as_slice()) {
                    Ok(string) => error!(?string, ERR_S),
                    _ => error!(?bytes, ERR_S),
                }
                Err(InitProtocolError::NotHandshake)
            },
            _ => {
                info!("Handshake failed");
                Err(InitProtocolError::NotHandshake)
            },
        }?;

        match sink.recv().await? {
            InitFrame::Init { pid, secret } => {
                debug!(?pid, "Participant send their ID");
                let stream_id_offset = if initializer {
                    STREAM_ID_OFFSET1
                } else {
                    drain
                        .send(InitFrame::Init {
                            pid: local_pid,
                            secret: local_secret,
                        })
                        .await?;
                    STREAM_ID_OFFSET2
                };
                info!(?pid, "This Handshake is now configured!");
                Ok((pid, stream_id_offset, secret))
            },
            InitFrame::Raw(bytes) => {
                match std::str::from_utf8(bytes.as_slice()) {
                    Ok(string) => error!(?string, ERR_S),
                    _ => error!(?bytes, ERR_S),
                }
                Err(InitProtocolError::NotId)
            },
            _ => {
                info!("Handshake failed");
                Err(InitProtocolError::NotId)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InitProtocolError, mpsc::test_utils::*};

    #[tokio::test]
    async fn handshake_drop_start() {
        let [mut p1, p2] = ac_bound(10, None);
        let r1 = tokio::spawn(async move { p1.initialize(true, Pid::fake(2), 1337).await });
        let r2 = tokio::spawn(async move {
            let _ = &p2;
            let _ = p2;
        });
        let (r1, _) = tokio::join!(r1, r2);
        assert_eq!(r1.unwrap(), Err(InitProtocolError::Custom(())));
    }

    #[tokio::test]
    async fn handshake_wrong_magic_number() {
        let [mut p1, mut p2] = ac_bound(10, None);
        let r1 = tokio::spawn(async move { p1.initialize(true, Pid::fake(2), 1337).await });
        let r2 = tokio::spawn(async move {
            let _ = p2.1.recv().await?;
            p2.0.send(InitFrame::Handshake {
                magic_number: *b"woopsie",
                version: VELOREN_NETWORK_VERSION,
            })
            .await?;
            let _ = p2.1.recv().await?;
            Result::<(), InitProtocolError<()>>::Ok(())
        });
        let (r1, r2) = tokio::join!(r1, r2);
        assert_eq!(
            r1.unwrap(),
            Err(InitProtocolError::WrongMagicNumber(*b"woopsie"))
        );
        assert_eq!(r2.unwrap(), Ok(()));
    }

    #[tokio::test]
    async fn handshake_wrong_version() {
        let [mut p1, mut p2] = ac_bound(10, None);
        let r1 = tokio::spawn(async move { p1.initialize(true, Pid::fake(2), 1337).await });
        let r2 = tokio::spawn(async move {
            let _ = p2.1.recv().await?;
            p2.0.send(InitFrame::Handshake {
                magic_number: VELOREN_MAGIC_NUMBER,
                version: [0, 1, 2],
            })
            .await?;
            let _ = p2.1.recv().await?;
            let _ = p2.1.recv().await?; //this should be closed now
            Ok(())
        });
        let (r1, r2) = tokio::join!(r1, r2);
        assert_eq!(r1.unwrap(), Err(InitProtocolError::WrongVersion([0, 1, 2])));
        assert_eq!(r2.unwrap(), Err(InitProtocolError::Custom(())));
    }

    /// APEX-T3.2/T3.3.05 (Opus 5's boundary-review finding): a pre-T3.2
    /// peer advertising the old minor version (`[0, 7, 0]`) is rejected --
    /// proves the live `VELOREN_NETWORK_VERSION` bump (0.7.0 -> 0.8.0,
    /// covering T3.2's and T3.3.05's cumulative wire changes) actually
    /// takes effect in the handshake, not just that the generic mismatch
    /// path works for an arbitrary version. Was T3.1.15's own 0.6.0->0.7.0
    /// bump test before this finding; this test's own tripwire assertion
    /// below is what caught the missed bump in the first place.
    #[tokio::test]
    async fn handshake_old_minor_version_rejected() {
        assert_eq!(VELOREN_NETWORK_VERSION, [0, 8, 0], "update this test's OLD_VERSION constant alongside any future bump");
        const OLD_VERSION: [u32; 3] = [0, 7, 0];
        let [mut p1, mut p2] = ac_bound(10, None);
        let r1 = tokio::spawn(async move { p1.initialize(true, Pid::fake(2), 1337).await });
        let r2 = tokio::spawn(async move {
            let _ = p2.1.recv().await?;
            p2.0.send(InitFrame::Handshake {
                magic_number: VELOREN_MAGIC_NUMBER,
                version: OLD_VERSION,
            })
            .await?;
            let _ = p2.1.recv().await?;
            let _ = p2.1.recv().await?;
            Ok(())
        });
        let (r1, r2) = tokio::join!(r1, r2);
        assert_eq!(r1.unwrap(), Err(InitProtocolError::WrongVersion(OLD_VERSION)));
        assert_eq!(r2.unwrap(), Err(InitProtocolError::Custom(())));
    }

    /// Same major/minor, different patch: still accepted (patch is ignored
    /// by the version gate -- packet section 3.6 / T3.1.15's "patch
    /// difference behavior pinned"). Manually drives both handshake steps
    /// (Handshake reply, then Init exchange) since the mock peer isn't
    /// running the real `initialize()` with the patched version.
    #[tokio::test]
    async fn handshake_patch_difference_still_accepted() {
        let mut patched_version = VELOREN_NETWORK_VERSION;
        patched_version[2] = patched_version[2].wrapping_add(1);
        let [mut p1, mut p2] = ac_bound(10, None);
        let r1 = tokio::spawn(async move { p1.initialize(true, Pid::fake(2), 1337).await });
        let r2 = tokio::spawn(async move {
            let _ = p2.1.recv().await?; // recv p1's Handshake
            p2.0.send(InitFrame::Handshake { magic_number: VELOREN_MAGIC_NUMBER, version: patched_version })
                .await?; // reply with the patched version
            match p2.1.recv().await? {
                InitFrame::Init { pid, secret } => {
                    p2.0.send(InitFrame::Init { pid: Pid::fake(3), secret: 42 }).await?;
                    Result::<_, InitProtocolError<()>>::Ok((pid, secret))
                },
                other => panic!("expected Init frame, got {other:?}"),
            }
        });
        let (r1, r2) = tokio::join!(r1, r2);
        assert_eq!(r1.unwrap(), Ok((Pid::fake(3), STREAM_ID_OFFSET1, 42)));
        assert_eq!(r2.unwrap(), Ok((Pid::fake(2), 1337)));
    }

    #[tokio::test]
    async fn handshake_unexpected_raw() {
        let [mut p1, mut p2] = ac_bound(10, None);
        let r1 = tokio::spawn(async move { p1.initialize(true, Pid::fake(2), 1337).await });
        let r2 = tokio::spawn(async move {
            let _ = p2.1.recv().await?;
            p2.0.send(InitFrame::Handshake {
                magic_number: VELOREN_MAGIC_NUMBER,
                version: VELOREN_NETWORK_VERSION,
            })
            .await?;
            let _ = p2.1.recv().await?;
            p2.0.send(InitFrame::Raw(b"Hello World".to_vec())).await?;
            Result::<(), InitProtocolError<()>>::Ok(())
        });
        let (r1, r2) = tokio::join!(r1, r2);
        assert_eq!(r1.unwrap(), Err(InitProtocolError::NotId));
        assert_eq!(r2.unwrap(), Ok(()));
    }
}
