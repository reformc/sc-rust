use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt, AsyncBufReadExt};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::broadcast::{self, Sender};
use std::error::Error;
use std::sync::Arc;
use tokio::io::BufReader;
use crate::{error::CustomizeError,gps};

fn parse_command(s:&str)->Result<Vec<Vec<String>>,Box<dyn Error>>{
    let s = s.replace("\r\n", ",");
    Ok(s.split(",").map(|cell|
        cell.split(" ").map(|c|c.to_string()).collect::<Vec<String>>()
    ).collect::<Vec<Vec<String>>>())
}
pub struct Client{
    pub sender:Arc<Sender<String>>,
    user:String,
    pass:String,
    addr:String,
    act_port:u16,
    server_name:String,
    command_id:usize
}

impl Client{
    pub fn new(user:&str,pass:&str,addr:&str)->Client{
        let (tx,_):(Sender<String>,_) = broadcast::channel(100);
        Client {
            sender:Arc::new(tx),
            user: user.to_string(),
            pass: pass.to_string(),
            addr: addr.to_string(),
            act_port: 8330,
            server_name: "".to_string(),
            command_id: 23177 }
    }

    pub async fn run(&mut self){
        loop{
            match self.connect().await{
                Ok(_)=>{},
                Err(e)=>println!("{}",e)
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    pub async fn connect(&mut self)->Result<(), Box<dyn Error>>{
        let sender = self.sender.clone();
        let mut stream = TcpStream::connect(&self.addr).await?;
        stream.write_all(format!("USER 23176, {}\r\n",&self.user).as_bytes()).await?;
        stream.readable().await?;
        let mut buf = [0;1024];
        let n = stream.try_read(&mut buf)?;
        let s = String::from_utf8((&buf[0..n]).to_vec())?;
        if !s.contains("Id is OK, authentication required"){
            return Err(CustomizeError::new(-1,&s));
        }
        stream.writable().await?;
        stream.write_all(format!("PASS 23177, {}\r\n",self.pass).as_bytes()).await?;
        let (read_half,write_half)= stream.into_split();
        let reader = BufReader::new(read_half);    
        let mut lines = reader.lines();
        tokio::spawn(Client::keepalive(write_half, self.command_id));
        loop{
            match lines.next_line().await? {
                Some(line)=>{
                    let command = parse_command(&line)?;
                    match &command[0][0] as &str{
                        "ACTPORT"=>{//设备信息端口
                            self.act_port = command[1][1].parse::<u16>()?;
                        },
                        "SEVERNAME"=>{//服务器名称，客户端使用
                            self.server_name = command[1][1].clone();
                        },
                        "200"=>{
                            continue;
                        },
                        "230"=>{//表示登录成功
                            //break;
                        },
                        "TCONFIG"=>{//gps信息
                            match gps::parse_gps(&line){
                                Ok(info)=>{
                                    let _ = sender.send(serde_json::to_string(&info).unwrap_or("".to_string()));//向广播通道发送
                                    log::info!("{:?}",info)
                                },
                                Err(e)=>{log::debug!("{}",e)}
                            }
                        }
                        "TERMREFRESH"=>{//设备上下线事件
                            match gps::parse_state(&line){
                                Ok(stat)=>{
                                    let _ = sender.send(serde_json::to_string(&stat).unwrap_or("".to_string()));//向广播通道发送
                                },
                                Err(e)=>{log::debug!("{}",e)}
                            }
                        }
                        _=>{}
                    }
                    log::debug!("{}",line)
                },
                None=>{
                    return Err(CustomizeError::new(-1, "recv None"));
                }
            }
        }
    }

    async fn keepalive(mut write_half:OwnedWriteHalf,mut command_id:usize){
        loop{
            tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
            command_id+=1;
            write_half.writable().await.unwrap();
            write_half.write(format!("CHB {},\r\n",command_id).as_bytes()).await.unwrap();
        }
    }
}


