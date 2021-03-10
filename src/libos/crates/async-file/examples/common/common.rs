use std::time::Instant;
use async_rt::task::JoinHandle;
use async_file::*;
use self::runtime::Runtime;

const FILE_LEN: usize = 1024 * 1024 * 400; // 160 MB
const FILE_NUM: usize = 1;
const BLOCK_SIZE: usize = 4096 * 16;
const LOOPS: usize = 1;

fn seq_read_write() {
    async_rt::task::block_on(async {
        let start = Instant::now();
        let mut join_handles: Vec<JoinHandle<i32>> = (0..FILE_NUM)
            .map(|i| {
                async_rt::task::spawn(async move {
                    
                    let file = {
                        let path = format!("tmp.data.{}", i).to_string();
                        let flags = libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC;
                        let mode = libc::S_IRUSR | libc::S_IWUSR;
                        AsyncFile::<Runtime>::open(path.clone(), flags, mode).unwrap()
                    };

                    let vec = vec![0; BLOCK_SIZE];
                    let mut buf = vec.into_boxed_slice();

                    for i in 0..LOOPS {
                        let mut offset = 0;
                    while offset < FILE_LEN {
                        buf[0] = buf[0] % 128 + 1;
                        let retval = file.write_at(offset, &buf[..]).await.unwrap();
                        assert!(retval >= 0);
                        offset += retval;
                    }

                        file.flush().await;
                    }
                    
                    
                    i as i32
                })
            })
            .collect();

        for (i, join_handle) in join_handles.iter_mut().enumerate() {
            assert!(join_handle.await == (i as i32));
        }

        let duration = start.elapsed();
        println!("async-file sequential write [file_size: {}, FILE_NUM: {}, BLOCK_SIZE: {}] is: {:?}, throughput: {} Mb/s", 
            FILE_LEN, FILE_NUM, BLOCK_SIZE, duration, ((FILE_LEN * FILE_NUM * LOOPS) as f64 / 1024.0 / 1024.0) / (duration.as_millis() as f64 / 1000.0));
        
    });


    async_rt::task::block_on(async {
        let start = Instant::now();
        let mut join_handles: Vec<JoinHandle<i32>> = (0..FILE_NUM)
            .map(|i| {
                async_rt::task::spawn(async move {
                    
                    let file = {
                        let path = format!("tmp.data.{}", i).to_string();
                        let flags = libc::O_RDWR;
                        let mode = 0;
                        AsyncFile::<Runtime>::open(path.clone(), flags, mode).unwrap()
                    };

                    for i in 0..LOOPS {

                    let vec = vec![0; BLOCK_SIZE];
                    let mut buf = vec.into_boxed_slice();
                    let mut offset = 0;
                    while offset < FILE_LEN {
                        let nbytes = file.read_at(offset, &mut buf[..]).await.unwrap();
                        assert!(nbytes > 0);
                        offset += nbytes as usize;
                    }
                }
                    
                    i as i32
                })
            })
            .collect();

        for (i, join_handle) in join_handles.iter_mut().enumerate() {
            assert!(join_handle.await == (i as i32));
        }

        let duration = start.elapsed();
        println!("async-file sequential read [file_size: {}, FILE_NUM: {}, BLOCK_SIZE: {}] is: {:?}, throughput: {} Mb/s", 
            FILE_LEN, FILE_NUM, BLOCK_SIZE, duration, ((FILE_LEN * FILE_NUM * LOOPS) as f64 / 1024.0 / 1024.0) / (duration.as_millis() as f64 / 1000.0));
        
    });
}

mod runtime {
    use std::sync::Once;

    use async_file::*;
    use async_rt::{waiter_loop, wait::WaiterQueue};
    use io_uring_callback::{IoUring, Builder};
    use lazy_static::lazy_static;

    pub struct Runtime;

    pub const PAGE_CACHE_SIZE: usize = 10240; // 40 MB
    pub const DIRTY_LOW_MARK: usize = PAGE_CACHE_SIZE / 10 * 3;
    pub const DIRTY_HIGH_MARK: usize = PAGE_CACHE_SIZE / 10 * 7;
    pub const MAX_DIRTY_PAGES_PER_FLUSH: usize = PAGE_CACHE_SIZE / 10;

    lazy_static! {
        static ref PAGE_CACHE: PageCache = PageCache::with_capacity(PAGE_CACHE_SIZE);
        static ref FLUSHER: Flusher<Runtime> = Flusher::new();
        static ref WAITER_QUEUE: WaiterQueue = WaiterQueue::new();
        pub static ref RING: IoUring = Builder::new().build(10240).unwrap();
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
    unsafe { ring.start_enter_syscall_thread(); }
    let callback = move || {
        ring.trigger_callbacks();
    };
    async_rt::config::set_sched_callback(callback);
}
