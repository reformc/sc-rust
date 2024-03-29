use crate::{error::CustomizeError, gps};
use std::error::Error;
use std::sync::Arc;
use tokio::io::BufReader;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedWriteHalf, OwnedReadHalf};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::broadcast::{self, Sender};

fn parse_command<'a>(s: &'a str) -> Vec<Vec<&'a str>> {
    s.split(",")
        .map(|cell| cell.split(" ").collect::<Vec<&str>>())
        .collect::<Vec<Vec<&str>>>()
}
pub struct Client {
    pub sender: Arc<Sender<String>>,//GPS广播通道
    user: String,//连接sc服务使用的用户名
    pass: String,//连接sc服务使用的密码
    addr: String,//sc服务地址
    act_port: u16,//sc服务的查询端口,默认为8330
    server_name: String,
    command_id: usize,
}

impl Client {
    pub fn new(user: &str, pass: &str, addr: &str) -> Client {
        let (tx, _): (Sender<String>, _) = broadcast::channel(100);
        Client {
            sender: Arc::new(tx),
            user: user.to_string(),
            pass: pass.to_string(),
            addr: addr.to_string(),
            act_port: 8330,
            server_name: "".to_string(),
            command_id: 23177,
        }
    }

    pub async fn run(&mut self) {
        loop {
            match self.connect().await {
                Ok(_) => {}
                Err(e) => log::error!("{}", e),
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        let mut stream = TcpStream::connect(&self.addr).await?;
        stream.write_all(format!("USER 23176, {}\r\n", &self.user).as_bytes()).await?;
        stream.readable().await?;
        let mut buf = [0; 1024];
        let n = stream.try_read(&mut buf)?;
        if n==0{
            return Err("receive none".into())
        }
        let s = String::from_utf8((&buf[0..n]).to_vec())?;
        if !s.contains("Id is OK, authentication required") {
            return Err(CustomizeError::new(-1, &s));
        }
        stream.writable().await?;
        stream.write_all(format!("PASS 23177, {}\r\n", self.pass).as_bytes()).await?;
        let (read_half, write_half) = stream.into_split();
        select! {
            res = Self::keepalive(write_half, self.command_id) => {
                if let Err(e)=res{
                    log::error!("{}",e);
                }
                log::info!("thread_keepalive exit");
            }
            res = self.main_stream(read_half) => {
                if let Err(e)=res{
                    log::error!("{}",e);
                }
                log::info!("thread_stream exit");
            }
        }
        Ok(())
    }

    async fn main_stream(&mut self,read_half:OwnedReadHalf,)->Result<(), Box<dyn Error+Send+Sync>>{
        let sender = self.sender.clone();
        let reader = BufReader::new(read_half);   
        let mut lines = reader.lines();
        loop {
            match lines.next_line().await? {
                Some(line) => {
                    let line = line.replace("\r\n", ",");
                    let command = parse_command(&line);
                    match &command[0][0] as &str {
                        "ACTPORT" => {
                            //设备信息端口
                            self.act_port = command[1][1].parse::<u16>()?;
                        }
                        "SEVERNAME" => {
                            //服务器名称，客户端使用
                            self.server_name = command[1][1].to_string();
                        }
                        "200" => {
                            continue;
                        }
                        "230" => { //表示登录成功
                             //break;
                        }
                        "TCONFIG" => {
                            //gps信息
                            match gps::parse_gps(&line) {
                                Ok(info) => {
                                    let _ = sender.send(
                                        serde_json::to_string(&info).unwrap_or("".to_string()),
                                    ); //向广播通道发送
                                    log::info!("{:?}", info)
                                }
                                Err(e) => {
                                    log::debug!("{}", e)
                                }
                            }
                        }
                        "TERMREFRESH" => {
                            //设备上下线事件
                            match gps::parse_state(&line) {
                                Ok(stat) => {
                                    let _ = sender.send(
                                        serde_json::to_string(&stat).unwrap_or("".to_string()),
                                    ); //向广播通道发送
                                    log::info!("{:?}", stat)
                                }
                                Err(e) => {
                                    log::debug!("{}", e)
                                }
                            }
                        }
                        _ => {}
                    }
                    log::debug!("{}", line)
                }
                None => {
                    return Err("recv None".into());
                }
            }
        }
    }

    async fn keepalive(mut write_half: OwnedWriteHalf, mut command_id: usize)->Result<(),Box<dyn std::error::Error+Send+Sync>> {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
            command_id += 1;
            write_half.writable().await?;
            write_half
                .write(format!("CHB {},\r\n", command_id).as_bytes())
                .await?;
        }
    }
}
