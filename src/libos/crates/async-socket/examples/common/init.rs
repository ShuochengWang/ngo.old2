use std::sync::Arc;

use async_socket::IoUringProvider;
use io_uring_callback::{Builder, IoUring};
use lazy_static::lazy_static;

lazy_static! {
    static ref RING: Arc<IoUring> = Arc::new(Builder::new().build(4096).unwrap());
}

struct IoUringInstanceType {}

impl IoUringProvider for IoUringInstanceType {
    type Instance = Arc<IoUring>;

    fn get_instance() -> Self::Instance {
        RING.clone()
    }
}

fn init_async_rt(parallelism: u32) {
    async_rt::config::set_parallelism(parallelism);

    let ring = RING.clone();
    unsafe { ring.start_enter_syscall_thread(); }
    let callback = move || {
        ring.trigger_callbacks();
    };
    async_rt::config::set_sched_callback(callback);
}
