use std::{net::SocketAddr, sync::Arc, convert::Infallible};
use tokio_stream::StreamExt as _ ;
use futures::stream::Stream;
use axum::{
    routing::get, 
    Router,  
    middleware::from_extractor, extract::{WebSocketUpgrade, ws::{WebSocket, Message}}, http::HeaderMap, response::{Response, Sse, sse::{Event, KeepAlive}, Html}, Json
};
use tokio::{sync::broadcast::Sender,time::{Duration,timeout}};
use crate::{web_auth, sc_auth::{self, DeviceGather}};

pub async fn run(port:u16,user:Arc<String>,addr:Arc<String>,sender:Arc<Sender<String>>){
    log::info!("web listen on {}",port);
    let app = Router::new()
    .route("/hzbit/video/gps-ws",get({//websocket服务
        let channel_sender = Arc::clone(&sender);
        move|headers,ws|channel_ws(headers,ws,channel_sender)
    }))
    .route("/hzbit/video/gps-sse",get({//sse服务
        let channel_sender = Arc::clone(&sender);
        move|headers|channel_sse(headers,channel_sender)
    }))
    .route("/hzbit/video/device",get(move||get_devices(user.clone(), addr.clone())))//获取设备信息接口
    .route_layer(from_extractor::<web_auth::RequireAuth>())//鉴权中间件
    .route("/hzbit/video/sse-test",get(sse_test))//sse调试页面
    .route("/hzbit/video/ws-test",get(ws_test))//ws调试页面
    .route("/",get(home));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    log::info!("listening on {}",addr);
    axum::Server::bind(&addr)
    .serve(app.into_make_service())
    .await
    .unwrap();
}

async fn get_devices(user:Arc<String>,addr:Arc<String>)->Json<DeviceGather>{
    Json(sc_auth::device_info(user, addr).await.unwrap())
}

async fn home()->String{
    "杭州比特视频监控设备事件接口".to_string()
}

async fn channel_sse(headers:HeaderMap,sender:Arc<Sender<String>>)->Sse<impl Stream<Item = Result<Event, Infallible>>>{
    let auth = headers.
    get(axum::http::header::AUTHORIZATION)
    .and_then(|v|v.to_str().ok())
    .map(|v|v.to_string())
    .unwrap_or("null".to_string());
    log::debug!("channel_sse connect,auth is {}",&auth);
    let receiver = sender.subscribe();
    let stream = futures::stream::unfold(receiver,|mut receiver|{
        async move{
            match receiver.recv().await{
                Ok(v)=>Some((Event::default().data(v),receiver)),
                Err(_)=>None
            }
        }
    }).map(Ok);
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn channel_ws(headers:HeaderMap,ws:WebSocketUpgrade,sender:Arc<Sender<String>>)->Response{
    ws.on_upgrade(move |socket|{handler_channel_ws(headers,socket, sender)})
}

async fn handler_channel_ws(headers:HeaderMap,mut socket:WebSocket,sender:Arc<Sender<String>>){
    let auth = headers.
    get(axum::http::header::AUTHORIZATION)
    .and_then(|v|v.to_str().ok())
    .map(|v|v.to_string())
    .unwrap_or("null".to_string());
    log::debug!("channel_ws connect,auth is {}",&auth);
    let mut receiver = sender.subscribe();
    socket.send(Message::Text("{\"msg\":\"success\"}".to_string())).await.unwrap();
    loop {
        match timeout(Duration::from_secs(10), receiver.recv()).await{
            Ok(rec)=>{
                match rec{
                    Ok(msg)=>{
                        if socket.send(Message::Text(msg)).await.is_err(){break}
                    }
                    Err(_)=>{break}
                } 
            },
            Err(_)=>{
                if socket.send(Message::Text("{\"msg\":\"keepalive\"}".to_string())).await.is_err(){
                    break;
                }
            }
        };
    }
}

async fn sse_test()->(HeaderMap,Html<&'static str>){
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::SET_COOKIE,
        format!("JSESSIONID={}","test").as_str().parse().unwrap(),);
    (
        headers,
Html(r#"<!DOCTYPE html>
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
        }
        window.addEventListener("load", init, false);
        </script>
    </head>
    <div id="output" style="height:800px;width:600px;resize:both;overflow:scroll;"></div>
</html>"#)
        )
}

async fn ws_test()->(HeaderMap,Html<&'static str>){
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::SET_COOKIE,
        format!("JSESSIONID={}","test").as_str().parse().unwrap(),);
    (
        headers,
Html(r#"<!DOCTYPE html>
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
        }
        </script>
    </head>
    <div id="output" style="height:800px;width:600px;resize:both;overflow:scroll;"></div>
</html>"#)
        )
}
