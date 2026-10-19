use ashpd::desktop::{
    screencast::{CursorMode, Screencast, SourceType, Stream as ScreencastStream},
    PersistMode, Session,
};
use std::os::fd::OwnedFd;

pub struct PortalHandle {
    pub fd: OwnedFd,
    pub node_id: u32,
    pub session: Session<'static, Screencast<'static>>,
}

/// screencast portal call
pub async fn open_screencast() -> Result<PortalHandle, String> {
    // We lekken de proxy om de Session 'static te maken.
    let proxy: &'static Screencast<'static> = Box::leak(Box::new(
        Screencast::new()
            .await
            .map_err(|e| format!("Screencast::new failed: {e}"))?,
    ));

    let session = proxy
        .create_session()
        .await
        .map_err(|e| format!("create_session failed: {e}"))?;

    proxy
        .select_sources(
            &session,
            CursorMode::Embedded,
            SourceType::Monitor | SourceType::Virtual,
            false,
            None,
            PersistMode::DoNot,
        )
        .await
        .map_err(|e| format!("select_sources failed: {e}"))?;

    let response = proxy
        .start(&session, None)
        .await
        .map_err(|e| format!("screencast start failed: {e}"))?
        .response()
        .map_err(|e| format!("screencast start response: {e}"))?;

    let stream: ScreencastStream = response
        .streams()
        .first()
        .ok_or_else(|| "Portal returned no streams".to_string())?
        .to_owned();

    let node_id = stream.pipe_wire_node_id();

    let fd = proxy
        .open_pipe_wire_remote(&session)
        .await
        .map_err(|e| format!("open_pipe_wire_remote failed: {e}"))?;

    Ok(PortalHandle {
        fd,
        node_id,
        session,
    })
}
