use async_socket::Socket;

async fn tcp_echo(port: u16) {
    let socket = Socket::<IoUringInstanceType>::new();
    
    {
        let servaddr = libc::sockaddr_in {
            sin_family: libc::AF_INET as u16,
            sin_port: port.to_be(),
            sin_addr: libc::in_addr { s_addr: 0 },
            sin_zero: [0; 8],
        };
        let ret = socket.bind(&servaddr);
        assert!(ret >= 0);
    }
    
    {
        let ret = socket.listen(100);
        assert_eq!(ret, 0);
    }
    println!("listen 127.0.0.1:{}", port);

    loop {
        if let Ok(client) = socket.accept(None).await {
            async_rt::task::spawn(async move {
                let mut buf = vec![0u8; 64 * 1024];

                loop {
                    let bytes_read = client.read(buf.as_mut_slice()).await;

                    if bytes_read == 0 {
                        break;
                    } else if bytes_read < 0 {
                        println!("read() error. ret: {}", bytes_read);
                        break;
                    }

                    let bytes_write = client.write(&buf[..bytes_read as usize]).await;

                    if (bytes_write != bytes_read) {
                        println!("bytes_write != bytes_read, bytes_write: {}, bytes_read: {}", bytes_write, bytes_read);
                        break;
                    }
                }
            });
        } else {
            println!("accept() return err.");
        }
    }
}
