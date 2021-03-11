#!/bin/sh
benchmark(){
    # kill server and clients
    for pid in $(/bin/ps | grep "client" | awk '{print $1}'); do kill -9 $pid; done
    for pid in $(/bin/ps | grep "server" | awk '{print $1}'); do kill -9 $pid; done
    for pid in $(/bin/ps | grep "app" | awk '{print $1}'); do kill -9 $pid; done
    for pid in $(/bin/ps | grep "tcp_echo" | awk '{print $1}'); do kill -9 $pid; done

    ./server $port &
    sleep 5
    ./client 127.0.0.1 $port $block_size $client_num $req_num | tee -a "$file_name"

    # kill server and clients
    for pid in $(/bin/ps | grep "client" | awk '{print $1}'); do kill -9 $pid; done
    for pid in $(/bin/ps | grep "server" | awk '{print $1}'); do kill -9 $pid; done

    sleep 5

    cd ..
    cargo run --quiet --release --example tcp_echo > /dev/null 2>&1 &
    sleep 5
    cd ./network-bench/
    ./client 127.0.0.1 $port $block_size $client_num $req_num | tee -a "$file_name"

    # kill server and clients
    for pid in $(/bin/ps | grep "client" | awk '{print $1}'); do kill -9 $pid; done
    for pid in $(/bin/ps | grep "tcp_echo" | awk '{print $1}'); do kill -9 $pid; done

    sleep 5

    cd ../examples/sgx
    cd bin && ./app &
    sleep 5
    cd ../../network-bench
    ./client 127.0.0.1 $port $block_size $client_num $req_num  | tee -a "$file_name"

    # kill server and clients
    for pid in $(/bin/ps | grep "client" | awk '{print $1}'); do kill -9 $pid; done
    for pid in $(/bin/ps | grep "app" | awk '{print $1}'); do kill -9 $pid; done
}

gcc -O3 -pthread server.c -o server
gcc -O3 -pthread server_poll.c -o server_poll
gcc -O3 -pthread client.c -o client

cd ../examples/sgx
make
cd ../../network-bench

file_name="benchmark_result.txt"

port=3456
block_size=32768

req_num=200000
for client_num in 1 2 3 4 5 6 7 8 9 10
do
   benchmark
   sleep 10
done

req_num=150000
for client_num in 12 15 16 20 24 25 28 30
do
   benchmark
   sleep 10
done

req_num=100000
for client_num in 32 35 36 40 50 60 64
do
   benchmark
   sleep 10
done

req_num=50000
for client_num in 70 80 90 100 128 150 200 250
do
   benchmark
   sleep 10
done

req_num=20000
for client_num in 300 350 400 450 500 600 700 800 900 1000
do
   benchmark
   sleep 10
done