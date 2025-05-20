use std::sync::Arc;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;

pub async fn create_peer_connection() -> Result<RTCPeerConnection, webrtc::Error> {
    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_string()],
            ..Default::default()
        }],
        ..Default::default()
    };

    let api = APIBuilder::new().build();
    api.new_peer_connection(config).await
}

pub async fn setup_media_tracks(peer_connection: &RTCPeerConnection) -> Result<(), webrtc::Error> {
    let video_track = Arc::new(TrackLocalStaticRTP::new(
        RTCRtpCodecCapability {
            mime_type: "video/h264".to_string(),
            ..Default::default()
        },
        "video".to_string(),
        "webrtc-rs".to_string(),
    ));

    peer_connection.add_track(video_track.clone()).await?;

    let audio_track = Arc::new(TrackLocalStaticRTP::new(
        RTCRtpCodecCapability {
            mime_type: "audio/opus".to_string(),
            ..Default::default()
        },
        "audio".to_string(),
        "webrtc-rs".to_string(),
    ));

    peer_connection.add_track(audio_track.clone()).await?;

    Ok(())
}
#[allow(dead_code)]
pub async fn handle_screen_sharing(peer_connection: &RTCPeerConnection) -> Result<(), webrtc::Error> {
    let screen_track = Arc::new(TrackLocalStaticRTP::new(
        RTCRtpCodecCapability {
            mime_type: "video/h264".to_string(),
            ..Default::default()
        },
        "screen".to_string(),
        "webrtc-rs".to_string(),
    ));

    peer_connection.add_track(screen_track.clone()).await?;

    Ok(())
}
