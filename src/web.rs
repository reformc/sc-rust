use crate::sc_auth::{self, DeviceGather};
#[cfg(feature = "auth")]
use crate::web_auth;
#[cfg(feature = "auth")]
use axum::middleware::from_extractor;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    http::HeaderMap,
    response::{
        sse::{Event, KeepAlive},
        Html, Response, Sse,
    },
    routing::get,
    Json, Router,
};
use futures::stream::Stream;
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use tokio::{
    sync::broadcast::Sender,
    time::{timeout, Duration},
};
use tokio_stream::StreamExt as _;

pub async fn run(port: u16, user: Arc<String>, addr: Arc<String>, sender: Arc<Sender<String>>) {
    let app = Router::new()
        .route(
            "/hzbit/video/gps-ws",//websocket服务
            get({
                let channel_sender = Arc::clone(&sender);//GPS广播通道,由sc8310任务传入,跨线程调用使用Arc引用。
                move |headers, ws| channel_ws(headers, ws, channel_sender)
            }),
        )
        .route(
            "/hzbit/video/gps-sse",//sse服务
            get({
                let channel_sender = Arc::clone(&sender);
                move |headers| channel_sse(headers, channel_sender)
            }),
        )
        .route(
            "/hzbit/video/device",//获取设备信息接口
            get(move || get_devices(user.clone(), addr.clone())),
        );
    #[cfg(feature = "auth")]
    let app = app.route_layer(from_extractor::<web_auth::RequireAuth>());//鉴权中间件
    let app = app
        .route("/hzbit/video/sse-test", get(sse_test)) //sse demo页面
        .route("/hzbit/video/ws-test", get(ws_test)) //ws demo页面
        .route("/", get(home));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    log::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_devices(user: Arc<String>, addr: Arc<String>) -> Json<DeviceGather> {
    Json(sc_auth::device_info(user, addr).await.unwrap())
}

async fn home() -> String {
    "杭州比特视频监控设备事件接口".to_string()
}

async fn channel_sse(
    headers: HeaderMap,
    sender: Arc<Sender<String>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let auth = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string())
        .unwrap_or("null".to_string());
    log::debug!("channel_sse connect,auth is {}", &auth);
    let receiver = sender.subscribe();
    let stream = futures::stream::unfold(receiver, |mut receiver| async move {
        match receiver.recv().await {
            Ok(v) => Some((Event::default().data(v), receiver)),
            Err(_) => None,
        }
    })
    .map(Ok);
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn channel_ws(
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    sender: Arc<Sender<String>>,
) -> Response {
    ws.on_upgrade(move |socket| handler_channel_ws(headers, socket, sender))
}

async fn handler_channel_ws(
    headers: HeaderMap,
    mut socket: WebSocket,
    sender: Arc<Sender<String>>,
) {
    let auth = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string())
        .unwrap_or("null".to_string());
    log::debug!("channel_ws connect,auth is {}", &auth);
    let mut receiver = sender.subscribe();
    socket
        .send(Message::Text("{\"msg\":\"success\"}".to_string()))
        .await
        .unwrap();
    loop {
        match timeout(Duration::from_secs(10), receiver.recv()).await {
            Ok(rec) => match rec {
                Ok(msg) => {
                    if socket.send(Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            },
            Err(_) => {
                if socket
                    .send(Message::Text("{\"msg\":\"keepalive\"}".to_string()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        };
    }
}

async fn sse_test() -> (HeaderMap, Html<&'static str>) {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::SET_COOKIE,
        format!("JSESSIONID={}", "test").as_str().parse().unwrap(),
    );
    (
        headers,
        Html(
            r#"<!DOCTYPE html>
<html>
    <head>
        <script>
        function init(){
            var source = new EventSource("/hzbit/video/gps-sse",{
                headers: {
                    "AUTHORIZATION": "test"
                }
            });
            source.addEventListener("message",function(event){
                //console.log(event.data);
                writeToScreen(event.data);
            });
        }
        function writeToScreen(message) {
            var pre = document.createElement("span");
            pre.innerHTML = message;
            output.appendChild(pre);
            var children = output.childNodes;
            if(children.length>100){
                output.removeChild(output.firstChild);
            }
            var body_div = document.getElementById("output");
            body_div.scrollTop = body_div.scrollHeight;
        }
        window.addEventListener("load", init, false);
        </script>
    </head>
    <div id="output" style="height:800px;width:600px;resize:both;overflow:scroll;"></div>
</html>"#,
        ),
    )
}

async fn ws_test() -> (HeaderMap, Html<&'static str>) {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::SET_COOKIE,
        format!("JSESSIONID={}", "test").as_str().parse().unwrap(),
    );
    (
        headers,
        Html(
            r#"<!DOCTYPE html>
<html>
    <head>
        <script>
            const socket = new WebSocket("ws://" + window.location.host +"/hzbit/video/gps-ws");
            socket.withCredentials = true;
            socket.headers = {
                "AUTHORIZATION": "test"
            };
            socket.onmessage = function(event) {
                //console.log("Received data: " + event.data);                
                writeToScreen(event.data);
            }
            
        function writeToScreen(message) {
            var pre = document.createElement("span");
            pre.innerHTML = message;
            output.appendChild(pre);
            var children = output.childNodes;
            if(children.length>100){
                output.removeChild(output.firstChild);
            }
            var body_div = document.getElementById("output");
            body_div.scrollTop = body_div.scrollHeight;
        }
        </script>
    </head>
    <div id="output" style="height:800px;width:600px;resize:both;overflow:scroll;"></div>
</html>"#,
        ),
    )
}
