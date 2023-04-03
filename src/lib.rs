use clap::Parser;
use std::sync::Arc;
mod error;
mod gps;
mod sc8310;
mod sc_auth;
mod web;
mod web_auth;

#[derive(Parser)]
#[clap(
    author = "reform <reformgg@gmail.com>",
    version = "0.2.0",
    about = "sc数据转web、websocket、sse",
    long_about = "sc数据转web、websocket、sse"
)]
struct Args {
    /// 登录用户名
    #[clap(long, short, default_value = "admin")]
    user: String,
    /// 登录密码
    #[clap(long, short, default_value = "admin")]
    pass: String,
    /// 登录地址
    #[clap(long, short, default_value = "127.0.0.1:8310")]
    addr: String,
    ///web监听端口
    #[clap(long, short, default_value = "80")]
    web_port: u16,
    ///日志级别,trace,debug,info,warn,error五种级别，默认为info
    #[clap(long, short, default_value = "info")]
    log_level: String,
}

//使用单线程会占用更少的内存,若要使用多线程将(flavor = "current_thread")删除。
#[tokio::main(flavor = "current_thread")]
pub async fn run() {
    let args = Args::parse();
    match &args.log_level as &str {
        "trace" => simple_logger::init_with_level(log::Level::Trace).unwrap(),
        "debug" => simple_logger::init_with_level(log::Level::Debug).unwrap(),
        "info" => simple_logger::init_with_level(log::Level::Info).unwrap(),
        "warn" => simple_logger::init_with_level(log::Level::Warn).unwrap(),
        _ => simple_logger::init_with_level(log::Level::Error).unwrap(),
    }
    log::info!(
        "username:{},password:{},connect addr:{},log level:{}",
        &args.user,
        &args.pass,
        &args.addr,
        &args.log_level
    );
    let mut c = sc8310::Client::new(&args.user, &args.pass, &args.addr); //创建一个连接sc服务的对象
    let user = Arc::new(args.user.to_string()); //创建一个可跨线程使用的用户名弱引用。
    let addr = Arc::new(args.addr.replace("8310", "8330")); //sc服务请求设备信息的地址，跨线程使用的弱引用
    tokio::spawn(web::run(args.web_port, user, addr, c.sender.clone())); //启动一个web服务器，包含设备信息服务，websocket服务，sse服务
    c.run().await; //运行上面创建的sc服务，开始连接sc服务器并接收设备的数据。
}
