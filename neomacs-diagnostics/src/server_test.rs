use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt; // for `oneshot`

use crate::metrics::{FrameMetrics, GcMetrics, MetricsSnapshot};
use crate::server::{MetricsProvider, router};

fn fixed_provider() -> Arc<dyn MetricsProvider> {
    Arc::new(|| MetricsSnapshot {
        frame: FrameMetrics {
            presents: 42,
            ..Default::default()
        },
        gc: GcMetrics {
            collections: 3,
            ..Default::default()
        },
    })
}

#[tokio::test]
async fn metrics_route_returns_snapshot_json() {
    let app = router(fixed_provider());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["frame"]["presents"], 42);
    assert_eq!(v["gc"]["collections"], 3);
}

#[tokio::test]
async fn index_route_is_self_describing() {
    let app = router(fixed_provider());
    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["name"], "neomacs-diagnostics");
    assert!(v["endpoints"]["/metrics"].is_string());
}

#[tokio::test]
async fn live_route_emits_event_stream() {
    let app = router(fixed_provider());
    let resp = app
        .oneshot(Request::builder().uri("/live").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(ct.starts_with("text/event-stream"), "content-type was {ct}");

    // Read only the first SSE data frame; the stream is otherwise infinite.
    let mut body = resp.into_body();
    let frame = body
        .frame()
        .await
        .expect("at least one body frame")
        .expect("frame ok");
    let data = frame.into_data().expect("data frame");
    let text = String::from_utf8_lossy(&data);
    assert!(text.contains("data:"), "frame was {text}");
    assert!(text.contains("\"presents\":42"), "frame was {text}");
}

#[tokio::test]
async fn serve_on_listener_answers_metrics_over_tcp() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(fixed_provider());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream
        .write_all(b"GET /metrics HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    let text = String::from_utf8_lossy(&buf);
    assert!(text.contains("200 OK"), "response was {text}");
    assert!(text.contains("\"presents\":42"), "response was {text}");
}
