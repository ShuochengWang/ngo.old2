use self::runtime::Runtime;
use async_file::*;
use std::time::Instant;

static mut BLOCK_SIZE: usize = 4096 * 1;
const CACHE_SIZE: usize = 1024 * 50; // 1024 * 50: 4 MB * 50 (cache hit), or 10 (cache miss)
const FILE_LEN: usize = 1024 * 1024 * 100;
const FILE_NUM: usize = 1;
const LOOPS: usize = 1;
const USE_FLUSH: bool = false;
const USE_O_DIRECT: bool = true;

#[derive(Debug)]
enum IoType {
    SEQ_READ,
    SEQ_WRITE,
    RND_READ,
    RND_WRITE,
}

static mut SEED: u32 = 0;
fn get_random() -> u32 {
    unsafe {
        SEED = SEED * 1103515245 + 12345;
        let hi = SEED >> 16;
        SEED = SEED * 1103515245 + 12345;
        let lo = SEED >> 16;
        return (hi << 16) + lo;
    }
}

fn prepare() {
    async_rt::task::block_on(async {
        let file = {
            let path = format!("tmp.data.{}", 0).to_string();
            let flags = libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC;
            let mode = libc::S_IRUSR | libc::S_IWUSR;
            AsyncFile::<Runtime>::open(path.clone(), flags, mode).unwrap()
        };

        let vec = vec![0; 4096];
        let buf = vec.into_boxed_slice();

        let mut offset = 0;
        while offset < FILE_LEN {
            let retval = file.write_at(offset, &buf[..]).await.unwrap();
            offset += retval;
        }

        file.flush().await.unwrap();
    });
}

async fn bench(io_type: IoType) {
    let block_size = unsafe { BLOCK_SIZE };
    let (is_read, is_seq) = match io_type {
        IoType::SEQ_READ => (true, true),
        IoType::SEQ_WRITE => (false, true),
        IoType::RND_READ => (true, false),
        IoType::RND_WRITE => (false, false),
    };

    let file = {
        let path = format!("tmp.data.{}", 0).to_string();
        let mut flags = libc::O_RDWR;
        if USE_O_DIRECT {
            flags |= libc::O_DIRECT;
        }
        let mode = 0;
        AsyncFile::<Runtime>::open(path.clone(), flags, mode).unwrap()
    };

    let start = Instant::now();

    let vec = vec![0; block_size];
    let mut buf = vec.into_boxed_slice();
    unsafe {
        SEED = 0;
    }
    for _ in 0..LOOPS {
        if is_seq {
            let mut offset = 0;
            while offset < FILE_LEN {
                if is_read {
                    let nbytes = file.read_at(offset, &mut buf[..]).await.unwrap();
                    assert!(nbytes > 0);
                    offset += nbytes as usize;
                } else {
                    let val = buf[0] + 1;
                    for col in 0..buf.len() {
                        buf[col] = val;
                    }

                    let nbytes = file.write_at(offset, &buf[..]).await.unwrap();
                    assert!(nbytes > 0);
                    offset += nbytes;
                }
            }
        } else {
            let mut cnt = 0;
            let block_num = FILE_LEN / block_size;
            while cnt < FILE_LEN {
                let offset = (get_random() as usize % block_num) * block_size;

                if is_read {
                    let nbytes = file.read_at(offset, &mut buf[..]).await.unwrap();
                    assert!(nbytes > 0);
                    cnt += nbytes as usize;
                } else {
                    let val = buf[0] + 1;
                    for col in 0..buf.len() {
                        buf[col] = val;
                    }

                    let nbytes = file.write_at(offset, &buf[..]).await.unwrap();
                    assert!(nbytes > 0);
                    cnt += nbytes;
                }
            }
        }

        if !is_read && USE_FLUSH {
            file.flush().await.unwrap();
        }
    }

    let duration = start.elapsed();
    println!("async-file {:?} [cache_size: {} MB, file_size: {} MB, FILE_NUM: {}, BLOCK_SIZE: {}, LOOPS: {}, USE_FLUSH: {}, USE_O_DIRECT: {}] is: {:?}, throughput: {} Mb/s", 
        io_type, CACHE_SIZE * 4096 / 1024 / 1024, FILE_LEN / 1024 / 1024, FILE_NUM, block_size, LOOPS, USE_FLUSH, USE_O_DIRECT, duration, ((FILE_LEN * FILE_NUM * LOOPS) as f64 / 1024.0 / 1024.0) / (duration.as_millis() as f64 / 1000.0));

    file.flush().await.unwrap();
}

fn test_read_write() {
    prepare();

    use std::{thread, time};
    let sleep_time = time::Duration::from_millis(2000);

    for i in 1..9 {
        unsafe {
            BLOCK_SIZE = 4096 * i;
        }

        async_rt::task::block_on(bench(IoType::RND_READ));
        // async_rt::task::block_on(bench(IoType::RND_WRITE));
        // async_rt::task::block_on(bench(IoType::SEQ_READ));
        // async_rt::task::block_on(bench(IoType::SEQ_WRITE));

        thread::sleep(sleep_time);
    }
}

mod runtime {
    use super::*;
    use std::sync::Once;

    use async_rt::{wait::WaiterQueue, waiter_loop};
    use io_uring_callback::{Builder, IoUring};
    use lazy_static::lazy_static;

    pub struct Runtime;

    pub const IO_URING_SIZE: usize = 10240;
    pub const PAGE_CACHE_SIZE: usize = CACHE_SIZE;
    pub const DIRTY_LOW_MARK: usize = PAGE_CACHE_SIZE / 10 * 3;
    pub const DIRTY_HIGH_MARK: usize = PAGE_CACHE_SIZE / 10 * 7;
    pub const MAX_DIRTY_PAGES_PER_FLUSH: usize = IO_URING_SIZE / 10;

    lazy_static! {
        static ref PAGE_CACHE: PageCache = PageCache::with_capacity(PAGE_CACHE_SIZE);
        static ref FLUSHER: Flusher<Runtime> = Flusher::new();
        static ref WAITER_QUEUE: WaiterQueue = WaiterQueue::new();
        pub static ref RING: IoUring = Builder::new().build(IO_URING_SIZE as u32).unwrap();
    }

    impl AsyncFileRt for Runtime {
        fn io_uring() -> &'static IoUring {
            &RING
        }
        fn page_cache() -> &'static PageCache {
            &PAGE_CACHE
        }

        fn flusher() -> &'static Flusher<Self> {
            &FLUSHER
        }

        fn auto_flush() {
            static INIT: Once = Once::new();
            INIT.call_once(|| {
                async_rt::task::spawn(async {
                    let page_cache = &PAGE_CACHE;
                    let flusher = &FLUSHER;
                    let waiter_queue = &WAITER_QUEUE;
                    waiter_loop!(waiter_queue, {
                        // Start flushing when the # of dirty pages rises above the high watermark
                        if page_cache.num_dirty_pages() < DIRTY_HIGH_MARK {
                            continue;
                        }

                        // Stop flushing until the # of dirty pages falls below the low watermark
                        while page_cache.num_dirty_pages() > DIRTY_LOW_MARK {
                            flusher.flush(MAX_DIRTY_PAGES_PER_FLUSH).await;
                        }
                    });
                });
            });

            if PAGE_CACHE.num_dirty_pages() >= DIRTY_HIGH_MARK {
                WAITER_QUEUE.wake_all();
            }
        }
    }
}

fn init_async_rt() {
    async_rt::config::set_parallelism(1);

    let ring = &runtime::RING;
    unsafe {
        ring.start_enter_syscall_thread();
    }
    let callback = move || {
        ring.trigger_callbacks();
    };
    async_rt::config::set_sched_callback(callback);
}