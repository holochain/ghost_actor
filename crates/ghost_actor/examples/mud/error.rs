#[derive(Debug, thiserror::Error)]
pub enum MudError {
    #[error(transparent)]
    GhostError(#[from] ghost_actor::GhostError),

    #[error(transparent)]
    MpscSendError(
        #[from]
        tokio::sync::mpsc::error::SendError<(
            Vec<u8>,
            tokio::sync::oneshot::Sender<tokio::io::Result<()>>,
        )>,
    ),

    #[error(transparent)]
    OneshotRecvError(#[from] tokio::sync::oneshot::error::RecvError),

    #[error(transparent)]
    StdIoError(#[from] std::io::Error),
}
