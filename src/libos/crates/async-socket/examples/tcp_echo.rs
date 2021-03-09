include!("common/init.rs");
include!("common/tcp_echo.rs");

fn main() {
    let port: u16 = 3456;
    let parallelism: u32 = 1;

    init_async_rt(parallelism);

    async_rt::task::block_on(tcp_echo(port));
}
