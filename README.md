编译命令:
cargo build --release

交叉编译请使用docker和cross(cross安装命令：cargo install cross)工具，编译命令为：
cross build --release --target=mipsel-unknown-linux-musl    #编译为mipsel架构的路由器运行的程序
cross build --release --target=x86-unknown-linux-gnu    #编译为x86架构的linux运行程序

运行时需加参数，不加参数时将使用以下默认参数运行：
sc-rust --user admin --pass admin --addr 127.0.0.1:8310 --web-port 80 --log-level info
参数说明可以运行sc-rust -h查看。

运行后可以打开
http://127.0.0.1/hzbit/video/sse-test 查看sse连接的demo页面
http://127.0.0.1/hzbit/video/ws-test 查看websocket连接的demo页面(websocket如果连续十秒没有数据会发一条keepalive包,sse不发keepalive包)
http://127.0.0.1/hzbit/video/device 查看全部设备信息(返回json格式，此接口后台需多次调用视频服务器tcp接口，性能较差，不建议频繁使用)

运行在老版本windows可能会有依赖问题，可以在编译时添加以下配置文件编译为无依赖的单体程序
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]

将鉴权代码加入"auth" feature并默认引入，
若不需要鉴权代码，编译时加上参数 --no-default-features，例如：
cross build --target=mips-unknown-linux-musl --profile minimal --no-default-features