#!/bin/sh
benchmark(){
    ./file-io $thread_num $file_num $block_size $req_merge_num $file_total_size $is_read $is_seq $use_fsync $use_direct $loops

    sleep 2
}

benchmark2(){
    cd ..
    cargo run --release --example test_read_write
    cd file-io-bench/

    sleep 2

    cd ../examples/sgx/bin
    ./app
    cd ../../../file-io-bench/

    sleep 2
}

gcc -O3 -pthread file-io.c -o file-io

cd ../examples/sgx
make
cd ../../file-io-bench/

benchmark2

file_name="benchmark_result.txt"

thread_num=1
file_num=1
req_merge_num=1
file_total_size=100
loops=1
use_fsync=0
use_direct=0

# is_read=1
# {
    # loops=1000

    # is_seq=1
    # use_direct=0
    # for block_size in 4 8 12 16 20 24 28 32
    # do
    #    benchmark
    # done

    # is_seq=0
    # use_direct=0
    # for block_size in 4 8 12 16 20 24 28 32
    # do
    #    benchmark
    # done

    # loops=5

    # is_seq=1
    # use_direct=1
    # for block_size in 4 8 12 16 20 24 28 32
    # do
    #    benchmark
    # done

    # is_seq=0
    # use_direct=1
    # for block_size in 4 8 12 16 20 24 28 32
    # do
    #    benchmark
    # done
# }

# is_read=0
# {
#     loops=500

#     is_seq=1
#     use_direct=0
#     use_fsync=0
#     for block_size in 4 8 12 16 20 24 28 32
#     do
#        benchmark
#     done

#     is_seq=0
#     use_direct=0
#     use_fsync=0
#     for block_size in 4 8 12 16 20 24 28 32
#     do
#        benchmark
#     done

#     loops=100

#     is_seq=1
#     use_direct=0
#     use_fsync=1
#     for block_size in 4 8 12 16 20 24 28 32
#     do
#        benchmark
#     done

#     is_seq=0
#     use_direct=0
#     use_fsync=1
#     for block_size in 4 8 12 16 20 24 28 32
#     do
#        benchmark
#     done

#     loops=5

#     is_seq=1
#     use_direct=1
#     use_fsync=0
#     for block_size in 4 8 12 16 20 24 28 32
#     do
#        benchmark
#     done

#     is_seq=0
#     use_direct=1
#     use_fsync=0
#     for block_size in 4 8 12 16 20 24 28 32
#     do
#        benchmark
#     done
# }
