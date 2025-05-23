use crate::models::AppState;
use gstreamer as gst;
use gstreamer_rtsp_server as rtsp_server;
use tokio::sync::mpsc;
use std::thread;

pub struct RTSPServer {
    _app_state: AppState,
    stream_rx: mpsc::Receiver<(String, Vec<u8>)>,
}

impl RTSPServer {
    pub fn new(app_state: AppState, stream_rx: mpsc::Receiver<(String, Vec<u8>)>) -> Self {
        gst::init().unwrap();
        Self { _app_state: app_state, stream_rx }
    }

    pub async fn start(&mut self) {
        let server = rtsp_server::RTSPServer::new();
        let mounts = server.mount_points().unwrap();
        let factory = rtsp_server::RTSPMediaFactory::new();
        factory.set_launch("appsrc name=src ! rtph264pay name=pay0 pt=96");
        mounts.add_factory("/live", &factory);
        let id = server.attach(None).unwrap();
        // Запускаем приём и подачу из канала в GStreamer
        let mut rx = self.stream_rx.clone();
        thread::spawn(move || {
            while let Some((_, buffer)) = tokio::runtime::Handle::current().block_on(rx.recv()) {
                // TODO: push buffer в appsrc (через соответствующий gst::AppSrc)
            }
        });
        // RTSP-сервер работает в этом потоке
        let _ = gtk::main();
    }
}
