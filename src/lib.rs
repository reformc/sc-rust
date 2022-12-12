use std::sync::Arc;
use clap::Parser;
mod web_auth;
mod web;
mod sc8310;
mod error;
mod sc_auth;
mod gps;

#[derive(Parser)]
#[clap(
    author="reform <reformgg@gmail.com>", 
    version="0.1.1",
    about="sc数据转web",
    long_about = "sc数据转web和websocket"
)]
struct Args{
    /// 登录用户名
    #[clap(long,short,default_value = "admin")]
    user: String,
    /// 登录密码
    #[clap(long,short,default_value = "admin")]
    pass: String,
    /// 登录地址
    #[clap(long,short,default_value = "127.0.0.1:8310")]
    addr: String,
    ///web监听端口
    #[clap(long,short,default_value = "80")]
    web_port: u16,
    ///日志级别,trace,debug,info,warn,error五种级别，默认为info
    #[clap(long,short,default_value = "info")]
    log_level: String
}

#[tokio::main]
pub async fn run(){
    let args = Args::parse();
    match &args.log_level as &str{
        "trace"=>simple_logger::init_with_level(log::Level::Trace).unwrap(),
        "debug"=>simple_logger::init_with_level(log::Level::Debug).unwrap(),
        "info"=>simple_logger::init_with_level(log::Level::Info).unwrap(),
        "warn"=>simple_logger::init_with_level(log::Level::Warn).unwrap(),
        _=>simple_logger::init_with_level(log::Level::Error).unwrap()
    }
    log::info!("username:{},password:{},connect addr:{},log level:{}",&args.user,&args.pass,&args.addr,&args.log_level);
    let mut c = sc8310::Client::new(&args.user,&args.pass,&args.addr);
    let sender = c.sender.clone();
    let user=Arc::new(args.user.to_string());
    let addr= Arc::new(args.addr.replace("8310", "8330"));
    tokio::spawn(web::run(args.web_port,user,addr, sender));
    c.run().await;
}