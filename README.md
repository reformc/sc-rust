windows下编译为exe程序或linux下编译为linux程序:
carogo build --release

windows下可用docker编译为linux程序：
docker run -it --rm -v ${pwd}:/workdir -v ~/.cargo/git:/root/.cargo/git -v ~/.cargo/registry:/root/.cargo/registry registry.gitlab.com/rust_musl_docker/image:stable-latest cargo build --release -vv --target=x86_64-unknown-linux-musl

运行时需加参数，不加参数时将使用以下默认参数运行：
sc-rust --user admin --pass admin --addr 127.0.0.1:8310 --web-port 80 --log-level info
参数说明可以用-h(sc-rust -h)查看。

运行后可以打开
http://127.0.0.1/hzbit/video/sse-test 查看sse测试页面
http://127.0.0.1/hzbit/video/ws-test 查看websocket测试页面