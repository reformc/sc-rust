use std::{net::SocketAddr, sync::Arc};
use axum::{
    routing::get, 
    Router,  
    middleware::from_extractor, extract::{WebSocketUpgrade, ws::{WebSocket, Message}}, http::HeaderMap, response::Response
};
use tokio::{sync::broadcast::Sender,time::{Duration,timeout}};
use crate::{web_auth, sc_auth};

pub async fn run(port:u16,user:Arc<String>,addr:Arc<String>,sender:Arc<Sender<String>>){
    log::info!("web listen on {}",port);
    let app = Router::new()
    .route("/hzbit/video/gps-ws",get({
        let channel_sender = Arc::clone(&sender);
        move|headers,ws|channel_ws(headers,ws,channel_sender)
    }))
    .route("/hzbit/video/device",get({move||get_devices(user.clone(), addr.clone())
    }))
    .route_layer(from_extractor::<web_auth::RequireAuth>())//websocket通过header鉴权
    .route("/",get(home));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    log::info!("listening on {}",addr);
    axum::Server::bind(&addr)
    .serve(app.into_make_service())
    .await
    .unwrap();
}

async fn get_devices(user:Arc<String>,addr:Arc<String>)->String{
    match sc_auth::device_info(user, addr).await{
        Ok(devices)=>{serde_json::to_string(&devices).unwrap_or("".to_string())},
        Err(e)=>format!("{}",e)
    }
}

async fn home()->String{
    "杭州比特视频监控设备事件接口".to_string()
}

pub async fn channel_ws(headers:HeaderMap,ws:WebSocketUpgrade,sender:Arc<Sender<String>>)->Response{
    ws.on_upgrade(move |socket|{handler_channel_ws(headers,socket, sender)})
}

async fn handler_channel_ws(headers:HeaderMap,mut socket:WebSocket,sender:Arc<Sender<String>>){
    let auth = headers.
    get(axum::http::header::AUTHORIZATION)
    .and_then(|v|v.to_str().ok())
    .map(|v|v.to_string())
    .unwrap_or("null".to_string());
    log::info!("lock_open_ws connect,auth is {}",&auth);
    let mut receiver = sender.subscribe();
    socket.send(Message::Text("{\"msg\":\"success\"}".to_string())).await.unwrap();
    loop {
        match timeout(Duration::from_secs(6), receiver.recv()).await{
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
